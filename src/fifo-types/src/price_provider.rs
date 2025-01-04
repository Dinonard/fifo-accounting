use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::fmt::Debug;

use crate::{Transaction, AssetType};

/// Trait for providing the price of a token at a given time (date).
pub trait PriceProvider: Debug + Eq + PartialEq {
    /// Get the price of the given token at the given time.
    ///
    /// Returns the price as a `Decimal`, or an error message if the price is not available.
    fn get_price(&self, token: AssetType, date: NaiveDate) -> Result<Decimal, String>;

    /// Check if the price for the given token at the given date is available.
    fn contains_price(&self, token: AssetType, date: NaiveDate) -> bool {
        self.get_price(token, date).is_ok()
    }
}

/// Used to check whether there are missing prices in the price provider.
pub trait MissingPricesCheck {
    /// Check if there are any missing prices for the given transactions.
    ///
    /// Returns a list of tuples, where each tuple contains the asset type and the date for which the price is missing.
    /// If there are no missing prices, an empty list is returned.
    fn missing_prices(&self, transactions: &[Transaction]) -> Vec<(AssetType, NaiveDate)>;
}