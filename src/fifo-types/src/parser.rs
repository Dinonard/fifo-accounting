use crate::Transaction;

pub type ParserDataType = Result<Vec<Transaction>, Box<dyn std::error::Error>>;

/// 'Parse' the underlying data source and return a list of transactions.
///
/// Each transaction must be validated on its own, without any context of the previous transactions.
/// Expectations:
/// * Each transaction is valid on its own.
/// * Transactions are in order - dates have to be monotonically increasing.
pub trait DataParser: Iterator<Item = ParserDataType> {}
impl <T: Iterator<Item = ParserDataType>> DataParser for T {}

/// Provider of transactions for the FIFO calculation.
///
/// This type will acquire all of the transactions that need to be processed, and prepare them for further validation & processing.
pub struct TransactionsProvider<T: DataParser> {
    iter: T,
}

impl<T: DataParser> TransactionsProvider<T> {
    /// Consumes the `TransactionsProvider` and returns a list of all transactions, sorted by date.
    pub fn get(self) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
        let mut transactions = Vec::new();

        for entry in self.iter {
            transactions.append(&mut entry?);
        }

        // In case the files & sheets weren't provided in the correct order.
        transactions.sort_by_key(|t| t.date());

        // Update the ordinals.
        let mut counter: u32 = 0;
        transactions = transactions
            .into_iter()
            .map(|tx| {
                counter += 1;
                tx.new_with_ordinal(counter)
            })
            .collect();

        log::debug!("Parsed a total of {} transactions.", transactions.len());

        Ok(transactions)
    }
}

/// Convenience implementation.
impl<T: DataParser> From<T> for TransactionsProvider<T> {
    fn from(iter: T) -> Self {
        TransactionsProvider { iter }
    }
}