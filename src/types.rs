use chrono::NaiveDateTime;
use rust_decimal::Decimal;

use std::{fmt, str::FromStr};

/// Type of transactions that modify the balance of any asset in the 'ledger'.
#[derive(Debug)]
pub enum TransactionType {
    Invoice,
    Swap,
    Interest,
    Buying,
    Selling,
    Bridge,
    NFT,
    Transfer,
    Airdrop,
    Lock,
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
            "NFT" => Ok(TransactionType::NFT),
            "Transfer" => Ok(TransactionType::Transfer),
            "Airdrop" => Ok(TransactionType::Airdrop),
            "Lock" => Ok(TransactionType::Lock),
            _ => Err(()),
        }
    }
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Represents a single transaction that resulted in modification of the ledger.
pub struct Transaction {
    /// Ordinal number of the transaction in the ledger.
    ordinal: u32,
    /// Date on which the transaction was made.
    date: NaiveDateTime,
    /// Type of transaction (e.g. selling or a swap)
    tx_type: TransactionType,
    /// Type of the input token.
    input_type: String, // TODO: combine type with amount?
    /// Amount of the input token.
    input_amount: Decimal,
    /// Type of the output token.
    output_type: String,
    /// Amount of the output token.
    output_amount: Decimal,
    /// Free text note about the transaction.
    note: String,
}

impl Transaction {
    /// Create a new `Transaction` instance.
    pub fn new(
        ordinal: u32,
        date: NaiveDateTime,
        tx_type: TransactionType,
        input_type: String,
        input_amount: Decimal,
        output_type: String,
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
}
