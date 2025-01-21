use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Debug, str::FromStr};

use fifo_types::{AssetType, MissingPricesCheck, PriceProvider, Transaction};

// Big TODO:
// Implement a logic to fetch price from online price providers, like CoinGecko, CoinMarketCap, etc.
// Also automatically convert the price from USD value to EUR value.
// Doing this manually is cumbersome.

/// A basic solution for the 'price provider, which reads the prices from a file, and stores them in memory.
///
/// No dynamic updates are supported, and the prices are read once from the file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BasicPriceProvider {
    // prices: HashMap<(AssetType, NaiveDate), Decimal>,
    prices: HashMap<(AssetType, NaiveDate), Decimal>,
}

impl BasicPriceProvider {
    /// Create a new `BasicPriceProvider` from the configuration in the given file path.
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

            if prices_map.contains_key(&(token.clone(), date)) {
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
        match self.prices.get(&(token.clone(), date)) {
            Some(price) => Ok(*price),
            None => Err(format!(
                "Price not found for token '{:?}' at date '{}'",
                token, date
            )),
        }
    }
}

impl MissingPricesCheck for BasicPriceProvider {
    fn missing_prices(&self, transactions: &[Transaction]) -> Vec<(AssetType, NaiveDate)> {
        transactions
            .iter()
            .filter_map(|tx| {
                if tx.is_zero_cost() && !self.contains_price(tx.output().0, tx.date()) {
                    Some((tx.output().0, tx.date()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
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
