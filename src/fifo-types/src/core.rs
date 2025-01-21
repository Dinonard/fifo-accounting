use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
    ops::Deref,
};
use serde::Deserialize;
use chrono::NaiveDate;
use rust_decimal::Decimal;

/// Type of transactions that modify the balance of any asset in the 'ledger'.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TransactionType {
    /// Invoice paid via crypto. Treated as if EUR was exchanged for the asset.
    Invoice,
    /// One asset was swapped for another.
    Swap,
    /// Interest received for holding an asset.
    Interest,
    /// Buying an asset with fiat.
    Buying,
    /// Selling an asset for fiat.
    Selling,
    /// Asset was moved between two different blockchains, resulting in some tangible loss.
    Bridge,
    /// Non-fungible token was bought or sold.
    Nft,
    /// Asset was transferred between two wallets or CEXes, resulting in some tangible loss.
    Transfer,
    /// Asset was received as part of an airdrop.
    Airdrop,
    /// Asset was locked or unlocked in some protocol.
    Lock,
    /// Fees paid to execute transactions
    Fees,
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
            "bridge" => Ok(TransactionType::Bridge),
            "nft" => Ok(TransactionType::Nft),
            "transfer" => Ok(TransactionType::Transfer),
            "airdrop" => Ok(TransactionType::Airdrop),
            "lock" => Ok(TransactionType::Lock),
            "fees" => Ok(TransactionType::Fees),
            _ => Err(()),
        }
    }
}

impl TransactionType {
    /// Check if the transaction is 'zero-cost', meaning that some asset was acquired for **zero** fiat amount.
    pub fn is_zero_cost(&self) -> bool {
        matches!(self, Self::Interest | Self::Airdrop)
    }
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Represents an asset that can be traded or held in the 'ledger'.
/// E.g. ASTR or BTC or USD (fiat).
///
/// Asset type is always in uppercase.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct AssetType(String);
// TODO: change from enum to this struct resulted in adding lots of 'clone' calls
// which is ugly & inefficient. Come up with a better solution later.

impl AssetType {
    /// Check if the asset is a fiat currency.
    pub fn is_fiat(&self) -> bool {
        matches!(&self.0[..], "USD" | "EUR")
    }

    /// Check if the asset is a cryptocurrency.
    pub fn is_crypto(&self) -> bool {
        !self.is_fiat() && !self.0.is_empty()
    }

    /// Check if the asset is a stablecoin.
    pub fn is_stablecoin(&self) -> bool {
        matches!(&self.0[..], "USDC" | "USDT")
    }

    /// Consume self, return inner string.
    pub fn inner(self) -> String {
        self.0
    }

    // TODO: improvement idea - add some sort of getters for some asset types,
    // make them efficient (shouldn't be initialized each time?)
    #[allow(non_snake_case)]
    pub fn EUR() -> Self {
        AssetType("EUR".to_string())
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
    /// Type of transaction (e.g. selling or a swap)
    tx_type: TransactionType,
    /// Type of the input token.
    input_type: AssetType,
    /// Amount of the input token.
    input_amount: Decimal,
    /// Type of the output token.
    output_type: AssetType,
    /// Amount of the output token.
    output_amount: Decimal,
    /// Free text note about the transaction.
    note: String,
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
        note: String,
    ) -> Self {
        Transaction {
            ordinal,
            date,
            tx_type,
            input_type,
            input_amount,
            output_type,
            output_amount,
            note,
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

    /// Free text note about the transaction.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Cost basis of the transaction.
    /// This is the price at which the output token was acquired.
    /// E.g. if 1.5 BTC was bought for 750 USD, the cost basis is 500 USD.
    pub fn cost_basis(&self) -> Option<Decimal> {
        if self.output_amount == Decimal::ZERO {
            None
        } else {
            Some(self.input_amount / self.output_amount)
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

    /// Check if the transaction is 'zero-cost', meaning that some asset was
    /// acquired for **zero** fiat amount. E.g. interest received or an airdrop.
    pub fn is_zero_cost(&self) -> bool {
        self.tx_type.is_zero_cost()
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