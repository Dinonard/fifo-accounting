use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};

use super::types::AssetType;

/// Trait for providing the price of a token at a given time (date).
pub trait PriceProvider {
    /// Get the price of the given token at the given time.
    ///
    /// Returns the price as a `Decimal`, or an error message if the price is not available.
    fn get_price(&self, token: AssetType, date: NaiveDate) -> Result<Decimal, String>;

    /// Check if the price for the given token at the given date is available.
    fn contains_price(&self, token: AssetType, date: NaiveDate) -> bool {
        self.get_price(token, date).is_ok()
    }
}

/// A basic solution for the 'price provider, which reads the prices from a file, and stores them in memory.
///
/// No dynamic updates are supported, and the prices are read once from the file.
pub struct BasicPriceProvider {
    // prices: HashMap<(AssetType, NaiveDate), Decimal>,
    prices: HashMap<(AssetType, NaiveDate), Decimal>,
}

impl BasicPriceProvider {
    /// Create a new `BasicPriceProvider` from the given prices.
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let toml_content = std::fs::read_to_string(path)?;
        let prices: Prices = toml::from_str(&toml_content)?;

        let mut prices_map = HashMap::new();

        for Price { token, price, date } in prices.price {
            let token = AssetType::from_str(&token).map_err(|e| {
                format!(
                    "Failed to parse asset type: '{:?}', with error: {:?}",
                    token, e
                )
            })?;
            let date = NaiveDate::parse_from_str(&date, "%d-%b-%Y")
                .map_err(|e| format!("Failed to parse date: '{}', with error: {}", date, e))?;

            if prices_map.contains_key(&(token, date)) {
                log::warn!(
                    "Duplicate price entry for token '{:?}' and date '{}'",
                    token,
                    date
                );
            }

            prices_map.insert((token, date), price);
        }

        Ok(Self { prices: prices_map })
    }
}

impl PriceProvider for BasicPriceProvider {
    fn get_price(&self, token: AssetType, date: NaiveDate) -> Result<Decimal, String> {
        match self.prices.get(&(token, date)) {
            Some(price) => Ok(*price),
            None => Err(format!(
                "Price not found for token '{:?}' at date '{}'",
                token, date
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Price {
    token: String,
    price: Decimal,
    date: String,
}

#[derive(Debug, Deserialize)]
struct Prices {
    price: Vec<Price>,
}
