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
#[derive(Debug, Clone, Eq, PartialEq)]
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
    cost_basis: Option<Decimal>,
    /// Unit sale price of the asset, if it was sold.
    sale_price: Option<Decimal>,
    /// Parent transaction Id, if this item uses assets from another transaction.
    parent_tx: Option<usize>,
}

impl InventoryItem {
    /// Cost basis of the asset.
    fn cost_basis(&self) -> Option<Decimal> {
        self.cost_basis
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
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
            TransactionType::Selling
            | TransactionType::Fees
            | TransactionType::Nft
            | TransactionType::Swap
            | TransactionType::Lock => {
                self.process_selling_or_swap(transaction);
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
        let (output_token, output_amount) = transaction.output();

        // TODO: provide a dedicated function to handle inner ledger manipulation.
        // This should be especially useful when finding an entry.

        let entry = self.ledger.entry(output_token).or_insert_with(Vec::new);

        // Create a new inventory item for the transaction.
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

    /// Process a transaction which involves selling crypto for fiat or a swap.
    fn process_selling_or_swap(&mut self, transaction: Transaction) {
        let (input_token, input_amount) = transaction.input();
        let (output_token, _) = transaction.output();

        let inventory = self
            .ledger
            .get_mut(&input_token)
            .expect("Must exist since data was validated.");
        let mut remaining_amount = input_amount;

        let mut new_items = Vec::new();

        // TODO: need a more efficient way to start iteration. Use a dedicated function to provide an iterator.
        // There should be an 'last known index' to start from, to avoid iterating from the beginning.
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
            let new_cost_basis = if let Some(cost_basis) = item.cost_basis() {
                // If output is fiat, the cost basis remains the same.
                if output_token.is_fiat() {
                    Some(cost_basis)
                } else {
                    transaction.cost_basis().map(|cb| cb * cost_basis)
                }
            } else {
                None
            };

            let new_item = InventoryItem {
                ordinal: transaction.ordinal(),
                date: transaction.date(),
                amount: consumed_amount,
                remaining_amount: consumed_amount,
                // Chaining rule applies here.
                cost_basis: new_cost_basis,
                sale_price: transaction.sale_price(),
                parent_tx: Some(transaction.ordinal() as usize),
            };

            new_items.push(new_item);
        }

        // TODO: check if anything remains?
        if !remaining_amount.is_zero() {
            log::error!(
                "Remaining amount of {} for {:?} after processing transaction: {}",
                remaining_amount,
                input_token,
                transaction
            );

            for (key, value) in self.ledger.iter() {
                let amount = value.iter().map(|item| item.remaining_amount).sum::<Decimal>();
                log::error!("{:?}: {}", key, amount);
            }
        }

        // Add the new items to the ledger.
        self.ledger
            .entry(output_token)
            .or_insert_with(Vec::new)
            .extend(new_items);
    }

    /// Process a transaction which involves receiving interest or an airdrop.
    /// This is a zero cost basis transaction.
    fn process_interest(&mut self, transaction: Transaction) {
        let (output_token, output_amount) = transaction.output();

        self.ledger
            .entry(output_token)
            .or_insert_with(Vec::new)
            .push(InventoryItem {
                ordinal: transaction.ordinal(),
                date: transaction.date(),
                amount: output_amount,
                remaining_amount: output_amount,
                cost_basis: None,
                sale_price: None,
                parent_tx: None,
            });
    }

    // TODO: rethink how this is handled, this seems hacky.
    fn process_transfer(&mut self, transaction: Transaction) {
        let (input_type, input_amount) = transaction.input();
        let (_, output_amount) = transaction.output();

        let dummy_tx = Transaction::new(
            transaction.ordinal(),
            transaction.date(),
            transaction.tx_type(),
            input_type,
            input_amount - output_amount,
            AssetType::EUR,
            Decimal::ZERO,
            transaction.note().to_string(),
        );

        self.process_selling_or_swap(dummy_tx);
    }
}
