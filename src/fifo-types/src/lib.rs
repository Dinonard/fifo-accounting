
mod core;
mod parser;
mod price_provider;

pub use core::{TransactionType, AssetType, Transaction};
pub use parser::DataParser;
pub use price_provider::{PriceProvider, MissingPricesCheck};

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
