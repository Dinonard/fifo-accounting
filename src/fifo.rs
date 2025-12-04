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
//!
//! Repeat the process until the swap is satisfied.
//!
//! Each 'consumption' of the transaction is recoded as fragmentation.
//! For example, if a transaction has an output of 100 units, and a swap consumes 70 units, the transaction is fragmented into two parts:
//! 1. 70 units, consumed by the swap.
//! 2. 30 units, remaining for future swaps.
//!
//! The input amount of the original transaction & the output amount of the swap are fragmented in the same way.

use fifo_types::{AssetType, CsvLineData, Transaction, TransactionType};

use chrono::{Datelike, NaiveDate};
use itertools::Itertools;
use rust_decimal::Decimal;
use std::{
    borrow::Cow,
    cell::OnceCell,
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
    /// Whether the asset was acquired via interest.
    is_interest: bool,
}

impl InventoryItem {
    /// Cost basis of the asset.
    pub fn cost_basis(&self) -> Decimal {
        self.cost_basis
    }

    /// Income of the transaction.
    /// Equals the amount received in fiat (EUR).
    pub fn income(&self) -> Option<Decimal> {
        self.sale_price
            .map(|sale_price| sale_price * self.input_amount)
    }

    /// Expanse of the transaction.
    /// Equals the outflow of the value tied to the asset.
    pub fn expense(&self) -> Option<Decimal> {
        self.sale_price
            .map(|_sale_price| self.input_amount * self.cost_basis)
    }

    /// Profit of the transaction.
    /// If the asset was not sold yet, return `None`.
    pub fn profit(&self) -> Option<Decimal> {
        match (self.income(), self.expense()) {
            (Some(income), Some(expense)) => Some(income - expense),
            _ => None,
        }
    }

    /// Provide `CsvLineData` for the item.
    ///
    /// Transaction corresponding to the item must be provided.
    pub fn output_line(&self, tx: &Transaction) -> impl CsvLineData {
        assert_eq!(self.ordinal, tx.ordinal(), "Ordinal mismatch");

        #[derive(Debug)]
        struct CsvLine {
            ordinal: String,
            transaction_date: String,
            acquisition_date: String,
            action: String,
            input_type: String,
            input_amount: String,
            output_type: String,
            output_amount: String,
            income_amount: Option<String>,
            expense_amount: Option<String>,
            profit: Option<String>,
        }

        impl CsvLineData for CsvLine {
            fn ordinal(&self) -> Cow<str> {
                Cow::Borrowed(&self.ordinal)
            }

            fn transaction_date(&self) -> Cow<str> {
                Cow::Borrowed(&self.transaction_date)
            }

            fn acquisition_date(&self) -> Cow<str> {
                Cow::Borrowed(&self.acquisition_date)
            }

            fn action(&self) -> Cow<str> {
                Cow::Borrowed(&self.action)
            }

            fn input_type(&self) -> Cow<str> {
                Cow::Borrowed(&self.input_type)
            }

            fn input_amount(&self) -> Cow<str> {
                Cow::Borrowed(&self.input_amount)
            }

            fn output_type(&self) -> Cow<str> {
                Cow::Borrowed(&self.output_type)
            }

            fn output_amount(&self) -> Cow<str> {
                Cow::Borrowed(&self.output_amount)
            }

            fn income_amount(&self) -> Option<Cow<str>> {
                self.income_amount.as_deref().map(Cow::Borrowed)
            }

            fn expense_amount(&self) -> Option<Cow<str>> {
                self.expense_amount.as_deref().map(Cow::Borrowed)
            }

            fn profit(&self) -> Option<Cow<str>> {
                self.profit.as_deref().map(Cow::Borrowed)
            }
        }

        let ordinal = format!("{}", self.ordinal);
        let transaction_date = self.date.format("%d.%m.%Y").to_string();
        let acquisition_date = self.acquisition_date.format("%d.%m.%Y").to_string();
        let action = format!("{:?}", tx.tx_type());

        let input_type = format!("{}", tx.input().0);
        let input_amount = format!("{}", self.input_amount);

        let output_type = format!("{}", tx.output().0);
        let output_amount = format!("{}", self.output_amount);

        let income_amount = match self.income() {
            Some(income) => Some(format!("{}", income)),
            None => None,
        };

        let expense_amount = match self.expense() {
            Some(expense) => Some(format!("{}", expense)),
            None => None,
        };

        let profit = match self.profit() {
            Some(profit) => Some(format!("{}", profit)),
            None => None,
        };

        CsvLine {
            ordinal,
            transaction_date,
            acquisition_date,
            action,
            input_type,
            input_amount,
            output_type,
            output_amount,
            income_amount,
            expense_amount,
            profit,
        }
    }
}

// For easier readability
type Year = i32;

/// Yearly income & loss report, with remaining zero-cost assets per year.
struct YearlyReport {
    /// Year for which the report is generated.
    year: Year,
    /// Total income incurred by selling of assets.
    sell_income: Decimal,
    /// Total income incurred by interest.
    interest_income: Decimal,
    /// Total expense incurred by selling of assets.
    expense: Decimal,
}

impl YearlyReport {
    fn add_sell_income(&mut self, amount: Decimal) {
        self.sell_income = self
            .sell_income
            .checked_add(amount)
            .expect("Unexpected overflow.");
    }

    fn add_interest_income(&mut self, amount: Decimal) {
        self.interest_income = self
            .interest_income
            .checked_add(amount)
            .expect("Unexpected overflow.");
    }

    fn add_expense(&mut self, amount: Decimal) {
        self.expense = self
            .expense
            .checked_add(amount)
            .expect("Unexpected overflow.");
    }
}

impl Display for YearlyReport {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let profit = self
            .sell_income
            .checked_add(self.interest_income)
            .expect("Mustn't overflow")
            .checked_sub(self.expense)
            .expect("Mustn't underflow");

        write!(
            f,
            "Year {}: Sell Income: {:.2}, Interest Income: {:.2}, Expense: {:.2}, Profit: {:.2}",
            self.year, self.sell_income, self.interest_income, self.expense, profit,
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Ledger<'a> {
    /// List of all transactions, in order.
    transactions: Vec<Transaction>,
    /// Ledger of assets, used to keep track of the FIFO inventory.
    ledger: HashMap<AssetType, Vec<InventoryItem>>,
    /// Cache of the inventory items, sorted in order their respective transactions appear.
    /// Used to avoid sorting the items multiple times.
    in_order: OnceCell<Vec<&'a InventoryItem>>,
}

impl<'a> Ledger<'a> {
    /// Create a new `Ledger` instance.
    pub fn new(transactions: Vec<Transaction>) -> Self {
        let mut ledger = Ledger {
            transactions: Vec::new(), // ugly, maybe improve later
            ledger: HashMap::new(),
            in_order: OnceCell::new(),
        };

        ledger.process(&transactions);
        ledger.transactions = transactions;

        ledger
    }

    /// Vector of `InventoryItem` references, sorted in order their respective transactions appear.
    pub fn in_order(&'a self) -> &'a Vec<&'a InventoryItem> {
        self.in_order.get_or_init(|| {
            let mut items: Vec<_> = self
                .ledger
                .values()
                .flat_map(|asset_items| asset_items.iter())
                .collect();

            items.sort_by_key(|item| item.ordinal);
            items
        })
    }

    /// Iterator over the `CsvLineData` items, sorted in order.
    /// Should be used to generate the output CSV file.
    pub fn csv_line_iter(&'a self) -> impl Iterator<Item = impl CsvLineData + 'a> {
        self.in_order().iter().map(|item| {
            let tx = self.get_tx(item);
            item.output_line(tx)
        })
    }

    /// Yearly income & loss report.
    pub fn yearly_income_loss_report(&'a self) -> Vec<String> {
        let mut total_report = HashMap::<Year, YearlyReport>::new();

        for item in self.in_order() {
            let year = item.date.year();
            let report = total_report.entry(year).or_insert_with(|| YearlyReport {
                year,
                sell_income: Decimal::ZERO,
                interest_income: Decimal::ZERO,
                expense: Decimal::ZERO,
            });

            // If income from asset selling exists, add it to the report.
            if let Some(income) = item.income() {
                report.add_sell_income(income);
            }

            // If expense from asset selling exists, add it to the report.
            if let Some(expense) = item.expense() {
                report.add_expense(expense);
            }

            // If the item was acquired via interest, add its income to the report.
            if item.is_interest {
                report.add_interest_income(item.input_amount);
            }
        }

        total_report
            .into_iter()
            .sorted_by_key(|(year, _)| *year)
            .map(|(_, report)| format!("{}", report))
            .collect()
    }

    /// Get the transaction corresponding to the inventory item.
    ///
    /// The assumption is that inventory item is **valid**, i.e. that its ordinal matches
    /// an existing transaction.
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
            TransactionType::Buying | TransactionType::Invoice | TransactionType::Interest => {
                self.process_inflow(transaction);
            }
            TransactionType::Selling | TransactionType::Swap => {
                self.process_swap_or_outflow(transaction);
            }
        }
    }

    /// Process a transaction which involves acquiring new crypto assets.
    /// Input, regardless of the type, is always fiat (EUR).
    fn process_inflow(&mut self, transaction: &Transaction) {
        let (input_token, input_amount) = transaction.input();
        let (output_token, output_amount) = transaction.output();

        let entry = self.ledger.entry(output_token.clone()).or_default();

        // Create a new inventory item for the transaction.
        let item = InventoryItem {
            ordinal: transaction.ordinal(),
            date: transaction.date(),
            acquisition_date: transaction.date(),
            input_type: input_token,
            input_amount,
            output_type: output_token,
            output_amount,
            remaining_amount: output_amount,
            cost_basis: transaction
                .cost_basis()
                .expect("Validation ensures this is non-zero for Buy transaction."),
            sale_price: None,
            parent_tx: None,
            is_interest: transaction.tx_type() == TransactionType::Interest,
        };
        entry.push(item);
    }

    /// Process a transaction which involves selling crypto for fiat or a swap.
    fn process_swap_or_outflow(&mut self, transaction: &Transaction) {
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

            let new_cost_basis = if output_token.is_fiat() {
                item.cost_basis()
            } else {
                transaction
                    .cost_basis()
                    .expect("Validation must ensure that non-sell transactions have cost basis.")
                    * item.cost_basis()
            };

            let new_item = InventoryItem {
                ordinal: transaction.ordinal(),
                date: transaction.date(),
                acquisition_date: item.date,
                input_type: input_token.clone(),
                input_amount: consumed_amount,
                output_type: output_token.clone(),
                output_amount: new_amount,
                remaining_amount: new_amount,
                // Chaining rule applies here.
                cost_basis: new_cost_basis,
                sale_price: transaction.sale_price(),
                parent_tx: Some(transaction.ordinal() as usize),
                is_interest: false,
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
            .entry(output_token.clone())
            .or_default()
            .extend(new_items);
    }
}
