use crate::Transaction;

/// Trait for parsing the underlying data source and returning a list of transactions.
pub trait DataParser {
    /// 'Parse' the underlying data source and return a list of transactions.
    ///
    /// Each transaction must be validated on its own, without any context of the previous transactions.
    /// Only exception being that transactions must always be in order - dates have to be monotonically increasing.
    /// An error will be returned if the transaction is invalid for any reason.
    ///
    /// # Returns
    /// * `Vec<Transaction>` - List of transactions parsed from the data source.
    /// * `Box<dyn std::error::Error>` - If the data source could not be parsed.
    fn parse(&self) -> Result<Vec<Transaction>, Box<dyn std::error::Error>>;
}