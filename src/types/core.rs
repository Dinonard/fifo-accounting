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
use serde::Deserialize;
use std::{
    fmt::{self, Display, Formatter},
    ops::Deref,
    str::FromStr,
};

/// Type of transactions that modify the balance of any asset in the 'ledger'.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TransactionType {
    /// Invoice paid via crypto. Treated as if EUR was exchanged for the asset.
    Invoice,
    /// One asset was swapped for another.
    Swap,
    /// Interest received for holding an asset.
    Interest,
    /// Buy an asset with fiat.
    Buying,
    /// Sell an asset for fiat.
    Selling,
}

impl FromStr for TransactionType {
    type Err = ();

    fn from_str(input: &str) -> Result<TransactionType, Self::Err> {
        match input.to_lowercase().as_str() {
            "invoice" => Ok(TransactionType::Invoice),
            "swap" => Ok(TransactionType::Swap),
            "interest" => Ok(TransactionType::Interest),
            "buying" => Ok(TransactionType::Buying),
            "selling" => Ok(TransactionType::Selling),
            _ => Err(()),
        }
    }
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Represents an asset that can be traded or held in the 'ledger'.
/// E.g. ASTR or BTC or USD (fiat).
///
/// Asset type is always in uppercase.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct AssetType(String);
impl AssetType {
    /// Check if the asset is a fiat currency.
    /// Right now only EUR is supported as fiat.
    pub fn is_fiat(&self) -> bool {
        matches!(&self.0[..], "EUR")
    }

    /// Check if the asset is a cryptocurrency.
    pub fn is_crypto(&self) -> bool {
        !self.is_fiat() && !self.0.is_empty()
    }

    /// Consume self, return inner string.
    pub fn inner(self) -> String {
        self.0
    }
}

impl FromStr for AssetType {
    type Err = ();

    fn from_str(input: &str) -> Result<AssetType, Self::Err> {
        Ok(AssetType(input.to_uppercase().trim().to_string()))
    }
}

impl Deref for AssetType {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for AssetType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents a single transaction that resulted in modification of the ledger.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Transaction {
    /// Ordinal number of the transaction in the ledger.
    ordinal: u32,
    /// Date on which the transaction was made.
    date: NaiveDate,
    /// Type of transaction (e.g. Sell or a swap)
    tx_type: TransactionType,
    /// Type of the input token.
    input_type: AssetType,
    /// Amount of the input token.
    input_amount: Decimal,
    /// Type of the output token.
    output_type: AssetType,
    /// Amount of the output token.
    output_amount: Decimal,
    /// Transaction context, to help with error messages (e.g. filename, sheet, row).
    extra_info: String,
}

impl Transaction {
    /// Create a new `Transaction` instance.
    pub fn new(
        ordinal: u32,
        date: NaiveDate,
        tx_type: TransactionType,
        input_type: AssetType,
        input_amount: Decimal,
        output_type: AssetType,
        output_amount: Decimal,
        extra_info: String,
    ) -> Self {
        Transaction {
            ordinal,
            date,
            tx_type,
            input_type,
            input_amount,
            output_type,
            output_amount,
            extra_info,
        }
    }

    /// Ordinal number of the transaction in the sheet.
    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }

    /// Consume this transaction and create a new one with a different ordinal number.
    pub fn new_with_ordinal(mut self, ordinal: u32) -> Self {
        self.ordinal = ordinal;
        self
    }

    /// Date on which the transaction was made.
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// Type of the transaction.
    pub fn tx_type(&self) -> TransactionType {
        self.tx_type
    }

    /// Input token and amount.
    pub fn input(&self) -> (AssetType, Decimal) {
        (self.input_type.clone(), self.input_amount)
    }

    /// Output token and amount.
    pub fn output(&self) -> (AssetType, Decimal) {
        (self.output_type.clone(), self.output_amount)
    }

    /// Transaction context, to help with error messages (e.g. filename, sheet, row).
    pub fn extra_info(&self) -> &str {
        &self.extra_info
    }

    /// Cost basis of the transaction.
    /// This is the price at which the output token was acquired.
    /// E.g. if 1.5 BTC was bought for 750 USD, the cost basis is 500 USD.
    ///
    /// NOTE: In case output token is fiat, cost basis is 0.
    pub fn cost_basis(&self) -> Decimal {
        if self.output_amount == Decimal::ZERO {
            Decimal::ZERO
        } else {
            self.input_amount / self.output_amount
        }
    }

    /// Sale price of the transaction.
    /// This is the price at which the output token was sold.
    /// E.g. if 1.5 BTC was sold for 750 USD, the sale price is 500 USD.
    pub fn sale_price(&self) -> Option<Decimal> {
        if self.input_amount == Decimal::ZERO || !self.output_type.is_fiat() {
            None
        } else {
            Some(self.output_amount / self.input_amount)
        }
    }
}

impl Display for Transaction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let date_str = self.date.format("%d.%m.%Y").to_string();
        write!(
            f,
            "Transaction {}., {}: {} {:?} -> {} {:?} ({})",
            self.ordinal,
            date_str,
            self.input_amount,
            self.input_type,
            self.output_amount,
            self.output_type,
            self.tx_type
        )
    }
}
