//! First-In First-Out (FIFO) asset management system.
//!
//! This module provides FIFO asset management to help calculate the cost basis of assets.
//! It's used to calculate the capital gains for tax purposes.
//!
//! # Algorithm
//!
//! When asset is swapped for another asset, do the following:
//!
//! 1. Starting from the beginning of the list, find the first transaction that can satisfy the swap.
//!    Swap can be satisfied if output of the transaction is the same as the input of the swap.
//!    Transaction must have remaining _amount_ that can be _consumed_.
//!    Amount can only be consumed **once**.
//!
//! 2. Compare the amount of the transaction with the amount of the swap.
//!    a) In case the output amount equals that of the swap, consume it entirely.
//!    b) In case the output amount is greater than that of the swap, consume only the amount of the swap, leaving the rest for future swaps.
//!    c) In case the output amount is less than that of the swap, consume the entire output amount and continue looking for the next transaction.
//!       Repeat the process until the swap is satisfied.
//!
//! Each 'consumption' of the transaction is recoded as fragmentation.
//! For example, if a transaction has an output of 100 units, and a swap consumes 70 units, the transaction is fragmented into two parts:
//! 1. 70 units, consumed by the swap.
//! 2. 30 units, remaining for future swaps.
//!
//! The input amount of the original transaction & the output amount of the swap are fragmented in the same way.

use crate::types::{AssetType, Transaction, TransactionType};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Inventory item for the FIFO asset management system.
struct InventoryItem {
    /// Ordinal number of the transaction in the ledger.
    ordinal: u32,
    /// Date on which the transaction was made.
    date: NaiveDateTime,
    /// Output amount of the transaction.
    amount: Decimal,
    /// Remaining amount for 'consumption'.
    remaining_amount: Decimal,
    /// Cost basis of the asset, i.e. the price at which it was acquired.
    cost_basis: Decimal,
    /// Unit sale price of the asset, if it was sold.
    sale_price: Option<Decimal>,
    /// Parent transaction Id, if this item uses assets from another transaction.
    parent_tx: Option<usize>,
}

pub struct Ledger {
    /// Ledger of assets, used to keep track of the FIFO inventory.
    ledger: HashMap<AssetType, Vec<InventoryItem>>,
}

impl Ledger {
    /// Create a new `Ledger` instance.
    pub fn new() -> Self {
        Ledger {
            ledger: HashMap::new(),
        }
    }

    /// Process a list of transactions.
    ///
    /// Caller must ensure they are sorted, and are generally correct.
    pub fn process_transactions(&mut self, transactions: Vec<Transaction>) {
        for transaction in transactions {
            self.add_transaction(transaction);
        }
    }

    /// Add a new transaction to the ledger.
    fn add_transaction(&mut self, transaction: Transaction) {
        match transaction.tx_type() {
            TransactionType::Buying | TransactionType::Invoice => {
                self.process_buying(transaction);
            }
            TransactionType::Selling | TransactionType::Fees | TransactionType::Nft => {
                self.process_selling(transaction);
            }
            TransactionType::Swap | TransactionType::Lock => {
                self.process_swap(transaction);
            }
            TransactionType::Interest | TransactionType::Airdrop => {
                self.process_interest(transaction);
            }
            TransactionType::Transfer | TransactionType::Bridge => {
                self.process_transfer(transaction);
            }
        }
    }

    /// Process a transaction which involves trading fiat for crypto.
    fn process_buying(&mut self, transaction: Transaction) {
        let (input_token, input_amount) = transaction.input();
        let (output_token, output_amount) = transaction.output();

        // TODO: provide a dedicated function to handle inner ledger manipulation.
        // This should be especially useful when finding an entry.

        let entry = self.ledger.entry(output_token).or_insert_with(Vec::new);

        // Add the transaction to the ledger.
        let item = InventoryItem {
            ordinal: transaction.ordinal(),
            date: transaction.date(),
            amount: output_amount,
            remaining_amount: output_amount,
            cost_basis: transaction.cost_basis(),
            sale_price: None,
            parent_tx: None,
        };
        entry.push(item);
    }

    /// Process a transaction which involves selling crypto for fiat.
    fn process_selling(&mut self, transaction: Transaction) {
        let (input_token, input_amount) = transaction.input();
        let (output_token, output_amount) = transaction.output();

        if let Some(inventory) = self.ledger.get_mut(&input_token) {
            let mut remaining_amount = input_amount;

            // TODO: need a more efficient way to start iteration. Use a dedicated function to provide an iterator.
            for item in inventory.iter_mut() {
                if remaining_amount.is_zero() {
                    break;
                }

                let consumed_amount = if item.remaining_amount > remaining_amount {
                    // Consume the entire amount.
                    let consumed = remaining_amount;

                    item.remaining_amount -= remaining_amount;
                    remaining_amount = Decimal::ZERO;

                    consumed
                } else {
                    // Consume the remaining amount.
                    let consumed = item.remaining_amount;
                    remaining_amount -= item.remaining_amount;
                    item.remaining_amount = Decimal::ZERO;

                    consumed
                };

                // Add the transaction to the ledger.
                let item = InventoryItem {
                    ordinal: transaction.ordinal(),
                    date: transaction.date(),
                    amount: output_amount,
                    remaining_amount: output_amount,
                    cost_basis: transaction.cost_basis(),
                    sale_price: None,
                    parent_tx: None,
                };
                // TODO: continue here, get rest of the required data, add it to the fiat ledger.
            }
        } else {
            log::error!(
                "Token {:?} not found in the inventory ledger. Transaction: {}",
                input_token,
                transaction
            );
        }
    }

    fn process_swap(&mut self, transaction: Transaction) {}

    fn process_interest(&mut self, transaction: Transaction) {}

    fn process_transfer(&mut self, transaction: Transaction) {}
}
