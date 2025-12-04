// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::{hash_map::Entry, HashMap};

use fifo_types::{AssetType, Transaction, TransactionType};

/// Validate the transactions, and return the final state of the ledger.
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
pub fn context_validation(
    transactions: &Vec<Transaction>,
) -> Result<HashMap<AssetType, Decimal>, String> {
    let mut previous_ordinal = 0;
    let mut previous_date = NaiveDate::MIN;
    let mut state = HashMap::<AssetType, Decimal>::default();

    for tx in transactions {
        // 1. Validate the ordinal number.
        if tx.ordinal() != previous_ordinal + 1 {
            return Err(format!(
                "Context: {}; Ordinal number mismatch: expected {}, found {}",
                tx.extra_info(),
                previous_ordinal + 1,
                tx.ordinal()
            ));
        }
        previous_ordinal = tx.ordinal();

        // 2. Validate the date.
        if tx.date() < previous_date {
            return Err(format!(
                "Context {}; Date mismatch: expected >= {:?}, found {:?}",
                tx.extra_info(),
                previous_date,
                tx.date()
            ));
        }
        previous_date = tx.date();

        // 3. Execute the transaction.
        let (input_token, input_amount) = tx.input();
        let (output_token, output_amount) = tx.output();

        if input_amount.is_zero() {
            return Err(format!(
                "Context: {}; Input amount is zero for transaction: {:?}",
                tx.extra_info(),
                tx
            ));
        }

        // 3.1. Subtract the input amount in case it's not fiat.
        if input_token.is_crypto() {
            match state.entry(input_token.clone()) {
                Entry::Occupied(mut entry) => {
                    let entry = entry.get_mut();

                    if let Some(new_value) = entry.checked_sub(input_amount) {
                        if new_value < Decimal::ZERO {
                            return Err(format!(
                                "Context: {}; Negative balance of {} for {:?} after transaction: {:?}. State dump: {:?}",
                                tx.extra_info(),
                                new_value, input_token, tx, state
                            ));
                        }

                        *entry = new_value;
                    } else {
                        // This part should never happen, since `Decimal` supports negative numbers.
                        return Err(format!(
                            "Context: {}; Underflow for {:?} after transaction: {:?}",
                            tx.extra_info(),
                            input_token,
                            tx
                        ));
                    }
                }
                Entry::Vacant(_) => {
                    return Err(format!(
                        "Context: {}; Token {:?} not found in state for transaction: {:?}",
                        tx.extra_info(),
                        input_token,
                        tx
                    ));
                }
            }
        }

        // 3.2. Add the output amount in case it's not fiat.
        if output_token.is_crypto() {
            match state.entry(output_token.clone()) {
                Entry::Occupied(mut entry) => {
                    let entry = entry.get_mut();

                    let new_value = entry.checked_add(output_amount).ok_or_else(|| {
                        format!(
                            "Context: {}; Overflow for {:?} after transaction: {:?}.",
                            tx.extra_info(),
                            output_token,
                            tx
                        )
                    })?;

                    *entry = new_value;
                }
                Entry::Vacant(entry) => {
                    entry.insert(output_amount);
                }
            }
        }

        // 4. Specific tx type validation
        match tx.tx_type() {
            TransactionType::Interest => {
                validate_interest_transaction(tx)?;
            }
            TransactionType::Invoice => {
                validate_invoice_transaction(tx)?;
            }
            TransactionType::Swap => {
                validate_swap_transaction(tx)?;
            }
            TransactionType::Buying => {
                validate_buy_transaction(tx)?;
            }
            TransactionType::Selling => {
                validate_selling_transaction(tx)?;
            }
        }
    }

    Ok(state)
}

/// Validate interest transaction specifics.
fn validate_interest_transaction(tx: &Transaction) -> Result<(), String> {
    let (input_token, input_amount) = tx.input();
    let (output_token, output_amount) = tx.output();

    if !input_token.is_fiat() {
        return Err(format!(
            "Context: {}; Interest transaction should have fiat (EUR) input, found {:?} in transaction: {:?}",
            tx.extra_info(),
            input_token,
            tx
        ));
    }

    if input_amount.is_zero() {
        return Err(format!(
            "Context: {}; Interest transaction should have non-zero fiat input amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    if output_token.is_fiat() {
        return Err(format!(
            "Context: {}; Interest transaction does not support fiat output, found in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    if output_amount.is_zero() {
        return Err(format!(
            "Context: {}; Interest transaction should have non-zero output amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    Ok(())
}

fn validate_invoice_transaction(tx: &Transaction) -> Result<(), String> {
    let (input_token, input_amount) = tx.input();
    let (output_token, output_amount) = tx.output();

    if !input_token.is_fiat() {
        return Err(format!(
            "Context: {}; Invoice transaction should have fiat (EUR) input, found {:?} in transaction: {:?}",
            tx.extra_info(),
            input_token,
            tx
        ));
    }

    if input_amount.is_zero() {
        return Err(format!(
            "Context: {}; Invoice transaction should have non-zero fiat input amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    if output_token.is_fiat() {
        return Err(format!(
            "Context: {}; Invoice transaction does not support fiat output, found in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    if output_amount.is_zero() {
        return Err(format!(
            "Context: {}; Invoice transaction should have non-zero output amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    Ok(())
}

fn validate_swap_transaction(tx: &Transaction) -> Result<(), String> {
    let (input_token, input_amount) = tx.input();
    let (output_token, output_amount) = tx.output();

    if input_token.is_fiat() {
        return Err(format!(
            "Context: {}; Swap transaction should not have fiat input, found {:?} in transaction: {:?}",
            tx.extra_info(),
            input_token,
            tx
        ));
    }

    if input_amount.is_zero() {
        return Err(format!(
            "Context: {}; Swap transaction should have non-zero input amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    if output_token.is_fiat() {
        return Err(format!(
            "Context: {}; Swap transaction should not have fiat output, found {:?} in transaction: {:?}",
            tx.extra_info(),
            output_token,
            tx
        ));
    }

    if output_amount.is_zero() {
        return Err(format!(
            "Context: {}; Swap transaction should have non-zero output amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    if input_token == output_token {
        return Err(format!(
            "Context: {}; Swap transaction should have different input and output tokens, found {:?} in transaction: {:?}",
            tx.extra_info(),
            input_token,
            tx
        ));
    }

    Ok(())
}

fn validate_buy_transaction(tx: &Transaction) -> Result<(), String> {
    let (input_token, input_amount) = tx.input();
    let (output_token, output_amount) = tx.output();

    if !input_token.is_fiat() {
        return Err(format!(
            "Context: {}; Buy transaction should have fiat (EUR) input, found {:?} in transaction: {:?}",
            tx.extra_info(),
            input_token,
            tx
        ));
    }

    if input_amount.is_zero() {
        return Err(format!(
            "Context: {}; Buy transaction should have non-zero fiat input amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    if output_token.is_fiat() {
        return Err(format!(
            "Context: {}; Buy transaction should not have fiat output, found {:?} in transaction: {:?}",
            tx.extra_info(),
            output_token,
            tx
        ));
    }

    if output_amount.is_zero() {
        return Err(format!(
            "Context: {}; Buy transaction should have non-zero output amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    Ok(())
}

fn validate_selling_transaction(tx: &Transaction) -> Result<(), String> {
    let (input_token, input_amount) = tx.input();
    let (output_token, _output_amount) = tx.output();

    if input_token.is_fiat() {
        return Err(format!(
            "Context: {}; Sell transaction should not have fiat input, found {:?} in transaction: {:?}",
            tx.extra_info(),
            input_token,
            tx
        ));
    }

    if input_amount.is_zero() {
        return Err(format!(
            "Context: {}; Sell transaction should have non-zero input amount in transaction: {:?}",
            tx.extra_info(),
            tx
        ));
    }

    if !output_token.is_fiat() {
        return Err(format!(
            "Context: {}; Sell transaction should have fiat (EUR) output, found {:?} in transaction: {:?}",
            tx.extra_info(),
            output_token,
            tx
        ));
    }

    // It is ok to have zero output amount, that is used to represent things like fees.

    Ok(())
}
