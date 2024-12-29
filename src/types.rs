use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;

use std::{
    fmt::{self, Display, Formatter},
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
    /// Asset was locked in some protocol.
    Lock,
    /// Fees paid to execute transactions
    Fees,
}

impl FromStr for TransactionType {
    type Err = ();

    fn from_str(input: &str) -> Result<TransactionType, Self::Err> {
        match input {
            "Invoice" => Ok(TransactionType::Invoice),
            "Swap" => Ok(TransactionType::Swap),
            "Interest" => Ok(TransactionType::Interest),
            "Buying" => Ok(TransactionType::Buying),
            "Selling" => Ok(TransactionType::Selling),
            "Bridge" => Ok(TransactionType::Bridge),
            "NFT" => Ok(TransactionType::Nft),
            "Transfer" => Ok(TransactionType::Transfer),
            "Airdrop" => Ok(TransactionType::Airdrop),
            "Lock" => Ok(TransactionType::Lock),
            "Fees" => Ok(TransactionType::Fees),
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum AssetType {
    ASTR,
    SDN,
    USDC,
    USDT,
    BTC,
    ETH,
    ADA,
    USD,
    EUR,
    HAHA,
    LockedAstr,
    PINK,
    WBTC,
    WETH,
    EMPTY,
}

impl AssetType {
    /// Check if the asset is a fiat currency.
    pub fn is_fiat(&self) -> bool {
        matches!(self, Self::USD | Self::EUR)
    }

    /// Check if the asset is a cryptocurrency.
    pub fn is_crypto(&self) -> bool {
        !self.is_fiat() && *self != Self::EMPTY
    }

    /// Check if the asset is a stablecoin.
    pub fn is_stablecoin(&self) -> bool {
        matches!(self, Self::USDC | Self::USDT)
    }
}

impl FromStr for AssetType {
    type Err = ();

    fn from_str(input: &str) -> Result<AssetType, Self::Err> {
        // First make sure to:
        // 1. remove any '(fiat)' parts from the input string
        // 2. trim the input string
        // 3. convert the input string to uppercase
        let input = input
            .to_uppercase()
            .replace("(FIAT)", "")
            .trim()
            .to_string();

        match input.as_str() {
            "ASTR" => Ok(AssetType::ASTR),
            "SDN" => Ok(AssetType::SDN),
            "USDC" => Ok(AssetType::USDC),
            "USDT" => Ok(AssetType::USDT),
            "BTC" => Ok(AssetType::BTC),
            "ETH" => Ok(AssetType::ETH),
            "ADA" => Ok(AssetType::ADA),
            "USD" => Ok(AssetType::USD),
            "EUR" => Ok(AssetType::EUR),
            "HAHA" => Ok(AssetType::HAHA),
            "LOCKED ASTR" => Ok(AssetType::LockedAstr),
            "PINK" => Ok(AssetType::PINK),
            "WBTC" => Ok(AssetType::WBTC),
            "WETH" => Ok(AssetType::WETH),
            "EMPTY" => Ok(AssetType::EMPTY),
            _ => Err(()),
        }
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
        (self.input_type, self.input_amount)
    }

    /// Output token and amount.
    pub fn output(&self) -> (AssetType, Decimal) {
        (self.output_type, self.output_amount)
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

/// Contains data for a _single line_ in the output of the program.
/// This is used to generate a CSV file with the final state of the ledger.
pub struct OutputLine {
    pub ordinal: String,
    pub date: String,
    pub action: String,
    pub input_type: String,
    pub input_amount: String,
    pub output_type: String,
    pub output_amount: String,
    pub net_amount: String,
}

impl OutputLine {
    pub fn csv_header(delimiter: String) -> String {
        vec![
            "Ordinal",
            "Date",
            "Action",
            "Input Type",
            "Input Amount",
            "Output Type",
            "Output Amount",
            "Net Amount",
        ]
        .join(&delimiter)
    }

    pub fn to_csv_line(self, delimiter: String) -> String {
        vec![
            self.ordinal,
            self.date,
            self.action,
            self.input_type,
            self.input_amount,
            self.output_type,
            self.output_amount,
            self.net_amount,
        ]
        .join(&delimiter)
    }
}
