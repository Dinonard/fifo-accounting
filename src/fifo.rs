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

use crate::{
    price_provider::{BasicPriceProvider, PriceProvider},
    types::{AssetType, OutputLine, Transaction, TransactionType},
};
use chrono::{Datelike, NaiveDate};
use itertools::Itertools;
use rust_decimal::Decimal;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

/// Inventory item for the FIFO asset management system.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InventoryItem {
    /// Ordinal number of the transaction in the ledger.
    ordinal: u32,
    /// Date on which the transaction was made.
    date: NaiveDate,
    /// Date on which the acquisition of the origin asset was made.
    /// E.g. the date when origin asset was acquired via an invoice.
    acquisition_date: NaiveDate,
    /// Type of the input asset.
    input_type: AssetType,
    /// Input amount consumed from the transaction.
    input_amount: Decimal,
    /// Type of the output asset.
    output_type: AssetType,
    /// Output amount consumed from the transaction.
    output_amount: Decimal,
    /// Remaining amount for 'consumption'.
    remaining_amount: Decimal,
    /// Cost basis of the asset, i.e. the price at which it was acquired.
    cost_basis: Decimal,
    /// Unit sale price of the asset, if it was sold.
    sale_price: Option<Decimal>,
    /// Parent transaction Id, if this item uses assets from another transaction.
    parent_tx: Option<usize>,
    /// Whether the item is part of a 'sequence' of transactions which originated with a zero-cost basis transaction.
    is_zero_cost: bool,
}

impl InventoryItem {
    /// Check whether the inventory items is part of a 'sequence' of transactions
    /// which originated with a zero-cost basis transaction like an airdrop or interest.
    pub fn is_zero_cost(&self) -> bool {
        self.is_zero_cost
    }

    /// `true` if the asset was sold, `false` otherwise.
    pub fn is_sell(&self) -> bool {
        self.sale_price.is_some()
    }

    /// `true` if the item is the first in the sequence, `false` otherwise.
    ///
    /// First item in the sequence is the one that resulted in first acquisition of the asset.
    /// E.g. this can be an invoice, interest or airdrop.
    pub fn is_first_in_sequence(&self) -> bool {
        self.parent_tx.is_none()
    }

    /// Cost basis of the asset.
    pub fn cost_basis(&self) -> Decimal {
        self.cost_basis
    }

    /// Net amount from the sale of the asset.
    /// Can be either profit or loss.
    /// If the asset was not sold yet, return `None`.
    pub fn net_amount(&self) -> Option<Decimal> {
        match (self.is_zero_cost, self.cost_basis, self.sale_price) {
            // 1st scenario - asset was acquired for some price, and sold afterwards.
            (false, cost_basis, Some(sale_price)) => {
                Some(self.input_amount * (sale_price - cost_basis))
            }
            // 2nd scenario - asset was acquired as part of some airdrop or interest.
            (true, _, Some(sale_price)) => {
                Some(self.input_amount * sale_price)
            }
            // 3rd scenario - asset was acquired, but not sold yet.
            _ => None,
        }
    }

    /// Create an `OutputLine` from the inventory item.
    ///
    /// Vector of transactions used in processing must be provided. The order of transactions must be preserved.
    pub fn output_line(&self, transactions: &Vec<Transaction>) -> OutputLine {
        let tx = transactions
            .get(self.ordinal as usize - 1)
            .expect("Must exist since data was validated.");

        let ordinal = format!("{}", self.ordinal);
        let date = self.date.format("%d.%m.%Y").to_string();
        let action = format!("{:?}", tx.tx_type());

        let input_type = format!("{:?}", tx.input().0);
        let input_amount = format!("{}", self.input_amount);

        let output_type = format!("{:?}", tx.output().0);
        let output_amount = format!("{}", self.output_amount);

        let net_amount = self
            .net_amount()
            .map_or_else(|| String::from(""), |net_amount| format!("{}", net_amount));

        OutputLine {
            ordinal,
            date,
            action,
            input_type,
            input_amount,
            output_type,
            output_amount,
            net_amount,
        }
    }
}

// For easier readability
type Year = i32;

/// Yearly income & loss report, with remaining zero-cost assets per year.
struct YearlyReport {
    /// Year for which the report is generated.
    year: Year,
    /// Total income from selling any assets.
    income: Decimal,
    /// Total loss from selling any assets.
    loss: Decimal,
    /// Remaining zero-cost assets per year.
    /// If assets were sold, they won't be included here.
    remaining_zero_cost: HashMap<AssetType, Decimal>,
}

impl YearlyReport {
    /// Include net result amount in the report.
    /// Covers both income and loss.
    fn include_net_result(&mut self, amount: Decimal) {
        if amount.is_sign_positive() {
            self.income = self
                .income
                .checked_add(amount)
                .expect("Unexpected overflow.");
        } else {
            self.loss = self
                .loss
                .checked_add(amount)
                .expect("Unexpected underflow.");
        }
    }

    /// Include zero-cost asset in the report.
    /// Covers both asset acquisition and disposal.
    fn include_zero_cost(&mut self, asset_type: AssetType, amount: Decimal) {
        let entry = self
            .remaining_zero_cost
            .entry(asset_type)
            .or_insert_with(|| Decimal::ZERO);
        *entry += amount;
    }
}

impl Display for YearlyReport {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Year: {}\nTotal Income: {:.2}\nTotal Loss: {:.2}\nRemaining zero-cost assets: {:#?}",
            self.year, self.income, self.loss, self.remaining_zero_cost
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Ledger {
    /// List of all transactions, in order.
    transactions: Vec<Transaction>,
    /// Ledger of assets, used to keep track of the FIFO inventory.
    ledger: HashMap<AssetType, Vec<InventoryItem>>,
    /// Price provider used to fetch the price of assets.
    price_provider: BasicPriceProvider,
}

impl Ledger {
    /// Create a new `Ledger` instance.
    pub fn new(transactions: Vec<Transaction>, price_provider: BasicPriceProvider) -> Self {
        let mut ledger = Ledger {
            transactions: Vec::new(), // ugly, maybe improve later
            ledger: HashMap::new(),
            price_provider,
        };

        ledger.process(&transactions);
        ledger.transactions = transactions;

        ledger
    }

    /// Ledger of assets & transactions.
    pub fn ledger(&self) -> &HashMap<AssetType, Vec<InventoryItem>> {
        &self.ledger
    }

    /// Vector of output lines, sorted in order their respective transactions appear.
    pub fn output_lines(&self) -> Vec<OutputLine> {
        self.in_order()
            .iter()
            .map(|item| item.output_line(&self.transactions))
            .collect()
    }

    /// Vector of `InventoryItem` references, sorted in order their respective transactions appear.
    pub fn in_order(&self) -> Vec<&InventoryItem> {
        let mut items: Vec<_> = self
            .ledger
            .values()
            .flat_map(|asset_items| asset_items.iter())
            .collect();

        items.sort_by_key(|item| item.ordinal);
        items
    }

    // TODO
    pub fn yearly_income_loss_report(&self) -> String {
        let mut total_report = HashMap::<Year, YearlyReport>::new();

        for item in self.in_order() {
            let year = item.date.year();
            let mut report = total_report.entry(year).or_insert_with(|| YearlyReport {
                year,
                income: Decimal::ZERO,
                loss: Decimal::ZERO,
                remaining_zero_cost: HashMap::new(),
            });

            // In case item represents a _selling_ action, include the net amount.
            if let Some(net_amount) = item.net_amount() {
                report.include_net_result(net_amount);
            }

            // TODO: This is the wrong approach and won't work.
            // Some things are missing & need to be accounted for.
            //
            // Scenario |
            // ---------|
            // 1) 100 ASTR is received as interest in 2023.
            // 2) 60 ASTR is sold in 2023, and that's treated as pure income.
            // 3) Remaining 40 ASTR is treated as _income_ at the end of 2023, based on the end-of-year price.
            // 4) Remaining 40 ASTR is sold in 2024, and that's treated as income or loss, compared to what was
            //    recorded at the end of 2023.
            //    E.g. if it was valued 0.05$ at the EoY, and now it's sold for 0.06$, the _profit_ is 0.01$.
            //
            // What does this mean?
            // - We need to keep track of the _end-of-year_ prices for all assets to correctly calculate gain or loss.
            // - When swapping or selling asset, we need to check historically whether the input asset was acquired in previous year.

            // In case item represents zero-cost asset acquisition, track it.
            if item.is_zero_cost() && item.is_first_in_sequence() {
                report.include_zero_cost(item.output_type, item.output_amount);
            } else if item.is_zero_cost() && !item.is_sell() {
                report.include_zero_cost(item.output_type, item.output_amount);
                report.include_zero_cost(item.input_type, -item.input_amount);
            } else if item.is_zero_cost() && item.is_sell() {
                report.include_zero_cost(item.input_type, -item.input_amount);
            }
        }

        total_report
            .into_iter()
            .sorted_by_key(|(year, _)| *year)
            .map(|(year, report)| format!("{}\n------{}", year, report))
            .intersperse("\n".to_string())
            .collect()
    }

    fn get_tx(&self, item: &InventoryItem) -> &Transaction {
        self.transactions
            .get(item.ordinal as usize - 1)
            .expect("Must exist since data was validated.")
    }

    /// Process a list of transactions.
    ///
    /// Caller must ensure they are sorted, and are generally correct.
    fn process(&mut self, transactions: &Vec<Transaction>) {
        for transaction in transactions {
            self.add_transaction(transaction);
        }
    }

    /// Add a new transaction to the ledger.
    fn add_transaction(&mut self, transaction: &Transaction) {
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
    fn process_buying(&mut self, transaction: &Transaction) {
        let (input_token, input_amount) = transaction.input();
        let (output_token, output_amount) = transaction.output();

        // TODO: provide a dedicated function to handle inner ledger manipulation.
        // This should be especially useful when finding an entry.

        let entry = self.ledger.entry(output_token).or_insert_with(Vec::new);

        // Create a new inventory item for the transaction.
        let item = InventoryItem {
            ordinal: transaction.ordinal(),
            date: transaction.date(),
            acquisition_date: transaction.date(),
            input_type: input_token,
            input_amount,
            output_type: output_token,
            output_amount: output_amount,
            remaining_amount: output_amount,
            cost_basis: transaction
                .cost_basis()
                .expect("Must be non-zero for buying transaction."),
            sale_price: None,
            parent_tx: None,
            is_zero_cost: false,
        };
        entry.push(item);
    }

    /// Process a transaction which involves selling crypto for fiat or a swap.
    fn process_selling_or_swap(&mut self, transaction: &Transaction) {
        let (input_token, input_amount) = transaction.input();
        let (output_token, output_amount) = transaction.output();

        let inventory = self
            .ledger
            .get_mut(&input_token)
            .expect("Must exist since data was validated.");
        let mut remaining_input_amount = input_amount;
        let mut remaining_output_amount = output_amount;

        let mut new_items = Vec::new();

        // TODO: need a more efficient way to start iteration. Use a dedicated function to provide an iterator.
        // There should be an 'last known index' to start from, to avoid iterating from the beginning.
        for item in inventory
            .iter_mut()
            .filter(|item| item.remaining_amount > Decimal::ZERO)
        {
            if remaining_input_amount.is_zero() {
                break;
            }

            let consumed_amount = if item.remaining_amount > remaining_input_amount {
                // Consume the entire amount.
                let consumed = remaining_input_amount;
                item.remaining_amount -= consumed;
                remaining_input_amount = Decimal::ZERO;

                consumed
            } else {
                // Consume the remaining amount.
                let consumed = item.remaining_amount;
                remaining_input_amount -= item.remaining_amount;
                item.remaining_amount = Decimal::ZERO;

                consumed
            };

            // Once remaining input amount reaches zero, consume the entire remaining output amount.
            let new_amount = if remaining_input_amount.is_zero() {
                remaining_output_amount
            } else {
                let new_amount = output_amount * consumed_amount / input_amount;
                remaining_output_amount -= new_amount;

                new_amount
            };

            // TODO: docs
            let new_cost_basis = if output_token.is_fiat() {
                item.cost_basis()
            } else {
                transaction
                    .cost_basis()
                    .expect("Cannot fail, improve later")
                    * item.cost_basis()
            };

            let new_item = InventoryItem {
                ordinal: transaction.ordinal(),
                date: transaction.date(),
                acquisition_date: item.date,
                input_type: input_token,
                input_amount: consumed_amount,
                output_type: output_token,
                output_amount: new_amount,
                remaining_amount: new_amount,
                // Chaining rule applies here.
                cost_basis: new_cost_basis,
                sale_price: transaction.sale_price(),
                parent_tx: Some(transaction.ordinal() as usize),
                is_zero_cost: item.is_zero_cost(),
            };

            new_items.push(new_item);
        }

        if !remaining_input_amount.is_zero() {
            log::error!(
                "Remaining amount of {} for {:?} after processing transaction: {}",
                remaining_input_amount,
                input_token,
                transaction
            );
        }

        // Add the new items to the ledger.
        self.ledger
            .entry(output_token)
            .or_insert_with(Vec::new)
            .extend(new_items);
    }

    /// Process a transaction which involves receiving interest or an airdrop.
    /// This is a zero cost basis transaction.
    fn process_interest(&mut self, transaction: &Transaction) {
        let (output_token, output_amount) = transaction.output();

        self.ledger
            .entry(output_token)
            .or_insert_with(Vec::new)
            .push(InventoryItem {
                ordinal: transaction.ordinal(),
                date: transaction.date(),
                acquisition_date: transaction.date(),
                input_type: AssetType::EUR,
                input_amount: Decimal::ZERO,
                output_type: output_token,
                output_amount: output_amount,
                remaining_amount: output_amount,
                cost_basis: self
                    .price_provider
                    .get_price(output_token, transaction.date())
                    .expect("Must exist"),
                sale_price: None,
                parent_tx: None,
                is_zero_cost: true,
            });
    }

    // TODO: rethink how this is handled, this seems hacky.
    fn process_transfer(&mut self, transaction: &Transaction) {
        let (input_type, input_amount) = transaction.input();
        let (output_type, output_amount) = transaction.output();

        if input_type == output_type {
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

            self.process_selling_or_swap(&dummy_tx);
        } else {
            self.process_selling_or_swap(transaction);
        }
    }
}
