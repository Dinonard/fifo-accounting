use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Type of transactions that modify the balance of any asset in the 'ledger'.
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

/// Represents a single transaction that resulted in modification of the ledger.
pub struct Transaction {
    /// Date on which the transaction was made.
    date: DateTime<Utc>,
    /// Type of transaction (e.g. selling or a swap)
    tx_type: TransactionType,
    input_type: String, // TODO: combine type with amount?
    input_amount: Decimal,
    output_type: String,
    output_amount: Decimal,
    /// Free text note about the transaction.
    note: String,
}
