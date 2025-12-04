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

use std::borrow::Cow;

/// Trait for the data that will be written to the CSV file.
///
/// Provides data for a single line in the CSV file.
pub trait CsvLineData {
    /// Overall ordinal of the transaction.
    fn ordinal(&self) -> Cow<str>;

    /// Date of the transaction.
    fn transaction_date(&self) -> Cow<str>;

    /// Date of acquisition of the asset.
    fn acquisition_date(&self) -> Cow<str>;

    /// Action taken in the transaction (e.g. swap or sell).
    fn action(&self) -> Cow<str>;

    /// Type of asset used as input in the transaction.
    fn input_type(&self) -> Cow<str>;

    /// Amount of asset used as input in the transaction.
    fn input_amount(&self) -> Cow<str>;

    /// Type of asset received as output in the transaction.
    fn output_type(&self) -> Cow<str>;

    /// Amount of asset received as output in the transaction.
    fn output_amount(&self) -> Cow<str>;

    /// Income amount.
    /// `None` if the transaction doesn't exchange asset for fiat.
    fn income_amount(&self) -> Option<Cow<str>>;

    /// Expense amount.
    /// `None` if the transaction doesn't exchange asset for fiat.
    fn expense_amount(&self) -> Option<Cow<str>>;

    /// Profit amount.
    /// `None` if the transaction doesn't exchange asset for fiat.
    fn profit(&self) -> Option<Cow<str>>;
}

/// Helper for writing data to the CSV file.
///
/// Provides utility functions like converting data to a single line in the CSV file & generating the header.
pub struct CsvHelper<T: CsvLineData> {
    delimiter: String,
    _phantom: std::marker::PhantomData<T>,
}

impl <T: CsvLineData> CsvHelper<T> {
    const HEADER_ELEMENTS: [&'static str; 11] = [
        "Ordinal",
        "Transaction Date",
        "Acquisition Date",
        "Action",
        "Input Type",
        "Input Amount",
        "Output Type",
        "Output Amount",
        "Income Amount",
        "Expense Amount",
        "Profit",
    ];

    /// Create a new `CsvHelper` instance.
    pub fn new(delimiter: String) -> Self {
        Self {
            delimiter,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Delimiter used in the CSV file.
    fn delimiter(&self) -> &str {
        &self.delimiter
    }

    /// Discrete elements of the CSV header.
    pub fn csv_header_elements(&self) -> &[&str] {
        &Self::HEADER_ELEMENTS
    }

    /// Full CSV header, as a single string.
    pub fn csv_header(&self) -> String {
        self.csv_header_elements().join(&self.delimiter())
    }

    /// Convert the data to a single line in the CSV file.
    pub fn to_csv_line_elements(&self, data: T) -> Vec<String> {
        vec![
            data.ordinal().into_owned(),
            data.transaction_date().into_owned(),
            data.acquisition_date().into_owned(),
            data.action().into_owned(),
            data.input_type().into_owned(),
            data.input_amount().into_owned(),
            data.output_type().into_owned(),
            data.output_amount().into_owned(),
            data.income_amount().map(|x| x.into_owned()).unwrap_or_default(),
            data.expense_amount().map(|x| x.into_owned()).unwrap_or_default(),
            data.profit().map(|x| x.into_owned()).unwrap_or_default(),
        ]
    }

    /// Convert the data to a single line in the CSV file.
    pub fn to_csv_line(&self, data: T) -> String {
        self.to_csv_line_elements(data).join(&self.delimiter())
    }
}
