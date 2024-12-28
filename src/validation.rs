use calamine::{Data, DataType};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::{hash_map::Entry, HashMap};
use std::str::FromStr;

use crate::types::{AssetType, Transaction, TransactionType};

/// Validate the row data against the expected format, and return the `Transaction`.
/// Each row is validated on its own, without any context of the previous rows.
///
/// # Arguments
/// * `row` - A row of data. Should be in the appropriate format.
///
/// # Returns
/// * `Transaction` - If the row is valid, return the parsed transaction.
/// * `String` - If the row is invalid, return an error message.
pub fn parse_row(row: &[Data]) -> Result<Transaction, String> {
    if row.len() < 8 {
        return Err(format!("Row is too short, skipping: {:?}", row));
    }

    // Helper function to parse a float as a decimal
    fn parse_decimal(data: &Data) -> Result<Decimal, String> {
        if let Data::Float(_) = data {
            Decimal::from_str(
                &data
                    .as_string()
                    .expect("Float can be represented as string."),
            )
            .map_err(|e| format!("Cannot convert float to Decimal: {:?}", e))
        } else {
            Err(format!("Expected a decimal value, found: {:?}", data))
        }
    }

    // Helper function to parse a string
    fn parse_string(data: &Data) -> Result<&str, String> {
        if let Data::String(value) = data {
            Ok(value)
        } else {
            Err(format!("Expected a string, found: {:?}", data))
        }
    }

    // 1. Parse the ordinal value.
    let ordinal = match row[0] {
        Data::Float(value) if value.fract() == 0.0 => value as u32,
        _ => {
            return Err(format!(
                "First column must be an ordinal (integer), skipping: {:?}",
                row
            ))
        }
    };

    // 2. Parse the date.
    let date = match row[1] {
        Data::DateTime(date) => date,
        _ => return Err(format!("Second column must be a date, skipping: {:?}", row)),
    };
    let date = date
        .as_datetime()
        .ok_or_else(|| {
            format!(
                "Cannot convert second column date to `Datetime`, skipping: {:?}",
                row
            )
        })?
        .date();

    // 3. Parse the action type.
    let action_type = if let Data::String(value) = &row[2] {
        TransactionType::from_str(value).map_err(|_| {
            format!(
                "Third column must be a valid action type, skipping: {:?}",
                row
            )
        })?
    } else {
        return Err(format!(
            "Third column must be a string (action type), skipping: {:?}",
            row
        ));
    };

    // 4. Parse the input token.
    let input_token = AssetType::from_str(parse_string(&row[3])?).map_err(|_| {
        format!(
            "Fourth column must be a valid asset type, skipping: {:?}",
            row
        )
    })?;

    // 5. Parse the input amount.
    let input_amount = parse_decimal(&row[4])?;

    // 6. Parse the output token.
    let output_token = AssetType::from_str(parse_string(&row[5])?).map_err(|_| {
        format!(
            "Fifth column must be a valid asset type, skipping: {:?}",
            row
        )
    })?;

    // 7. Parse the output amount.
    let output_amount = parse_decimal(&row[6])?;

    // 8. Parse the note.
    let note = parse_string(&row[7])?;

    // 9. Parse the extra info, if present. Not important.
    let _maybe_extra_info = if let Some(Data::String(value)) = row.get(8) {
        Some(value)
    } else {
        None
    };

    Ok(Transaction::new(
        ordinal,
        date,
        action_type,
        input_token,
        input_amount,
        output_token,
        output_amount,
        note.to_string(),
    ))
}

/// Validate the transactions in the sheet, and return the final state of the ledger.
/// There are several checks performed:
/// 1. The ordinal number should be sequential, starting at one and increasing by one.
/// 2. The dates should be monotonically increasing.
/// 3. The input amount should be subtracted from the state, and shouldn't result in a negative balance
///    (with a small tolerance for floating point errors & missing fees entries).
/// 4. The output amount should be added to the state, without any overflow.
///
/// # Arguments
/// * `transaction` - A list of transactions to validate, in ascending order.
/// * `init_state` - Initial state of the ledger, before the first transaction is applied.
///
/// # Returns
/// * `HashMap<AssetType, Decimal>` - If the transactions are valid, return the final state of the ledger.
/// * `String` - If the transactions are invalid, return an error message.
pub fn validate_sheet(
    transaction: &Vec<Transaction>,
    init_state: HashMap<AssetType, Decimal>,
    sheet_name: &str,
) -> Result<HashMap<AssetType, Decimal>, String> {
    let mut previous_ordinal = 0;
    let mut previous_date = NaiveDate::default();
    let mut state = init_state;

    for tx in transaction {
        // 1. Validate the ordinal number.
        if tx.ordinal() != previous_ordinal + 1 {
            return Err(format!(
                "Ordinal number mismatch: expected {}, found {}",
                previous_ordinal + 1,
                tx.ordinal()
            ));
        }
        previous_ordinal = tx.ordinal();

        // 2. Validate the date.
        if tx.date() < previous_date {
            return Err(format!(
                "Date mismatch: expected >= {:?}, found {:?}",
                previous_date,
                tx.date()
            ));
        }
        previous_date = tx.date();

        // 3. Execute the transaction.
        let (input_token, input_amount) = tx.input();
        let (output_token, output_amount) = tx.output();

        // 3.1. Subtract the input amount in case it's not fiat.
        if input_token.is_crypto() {
            match state.entry(input_token) {
                Entry::Occupied(mut entry) => {
                    let entry = entry.get_mut();

                    if let Some(new_value) = entry.checked_sub(input_amount) {
                        // TODO: revise this later - tolerance should differ for different tokens
                        if new_value < Decimal::ZERO
                            && new_value > -Decimal::from_str("0.1").unwrap()
                        {
                            log::warn!(
                                "Sheet: {}; Negative balance of {} for {:?} after transaction: {:?}",
                                sheet_name,
                                new_value,
                                input_token,
                                tx
                            );
                        } else if new_value < Decimal::ZERO {
                            return Err(format!(
                                "Negative balance of {} for {:?} after transaction: {:?}",
                                new_value, input_token, tx
                            ));
                        }

                        *entry = new_value;
                    } else {
                        // This part should never happen, since `Decimal` supports negative numbers.
                        return Err(format!(
                            "Underflow for {:?} after transaction: {:?}",
                            input_token, tx
                        ));
                    }
                }
                Entry::Vacant(_) => {
                    return Err(format!(
                        "Token {:?} not found in state for transaction: {:?}",
                        input_token, tx
                    ));
                }
            }
        }

        // 3.2. Add the output amount in case it's not fiat.
        if output_token.is_crypto() {
            match state.entry(output_token) {
                Entry::Occupied(mut entry) => {
                    let entry = entry.get_mut();

                    let new_value = entry.checked_add(output_amount).ok_or_else(|| {
                        format!(
                            "Overflow for {:?} after transaction: {:?}",
                            output_token, tx
                        )
                    })?;

                    *entry = new_value;
                }
                Entry::Vacant(entry) => {
                    entry.insert(output_amount);
                }
            }
        }

        // Selling for fiat, but not EUR.
        // Log this as a warning, but don't fail the validation.
        if !matches!(output_token, AssetType::EUR) && output_token.is_fiat() {
            log::warn!(
                "Sheet: {}; Selling for non-EUR fiat {:?} in transaction: {:?}. Take the EUR value instead at the transaction date.",
                sheet_name,
                output_token,
                tx
            );
        }
    }

    Ok(state)
}

// TODO: add validation for each transaction type - e.g. if it's "Buying", then input amount must be greater than zero, etc.
