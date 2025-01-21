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

use fifo_types::{AssetType, Transaction};

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
            match state.entry(input_token.clone()) {
                Entry::Occupied(mut entry) => {
                    let entry = entry.get_mut();

                    if let Some(new_value) = entry.checked_sub(input_amount) {
                        // TODO: revise this later - tolerance should differ for different tokens
                        if new_value < Decimal::ZERO {
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
            match state.entry(output_token.clone()) {
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
        if output_token != AssetType::EUR() && output_token.is_fiat() {
            log::warn!(
                "Selling for non-EUR fiat {:?} in transaction: {:?}. Take the EUR value instead at the transaction date.",
                output_token,
                tx
            );
        }
    }

    Ok(state)
}

// TODO: add validation for each transaction type - e.g. if it's "Buying", then input amount must be greater than zero, etc.
