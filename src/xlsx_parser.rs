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

use calamine::{open_workbook, Data, DataType, Reader, Xlsx};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

use fifo_types::{AssetType, ParserDataType, Transaction, TransactionType};

/// Specification for the XLSX file to parse.
/// Defines path to the file, which sheet to read from, and from which row to start reading.
#[derive(Debug, Deserialize)]
pub struct XlsxFileEntry {
    /// Path to the XLSX file.
    file_path: String,
    /// Name of the sheet to read from.
    sheet_name: String,
    /// Row number from which to start reading the data.
    start_row: usize,
}

/// Implementation of the transaction provider for parsing XLSX files.
pub struct XlsxParser {
    entries: Vec<XlsxFileEntry>,
    index: usize,
}

impl XlsxParser {
    pub fn new(entries: Vec<XlsxFileEntry>) -> Self {
        Self { entries, index: 0 }
    }

    /// Parse the XLSX file and return the transactions from the specified sheet.
    fn parse_xlsx_file(
        entry: &XlsxFileEntry,
    ) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
        let XlsxFileEntry {
            ref file_path,
            ref sheet_name,
            start_row,
        } = entry;

        let mut workbook: Xlsx<_> = open_workbook(file_path)?;

        if let Ok(range) = workbook.worksheet_range(sheet_name) {
            let mut row_number = *start_row;
            let mut previous_date = NaiveDate::MIN;

            let file_name = file_path
                .split('/')
                .last()
                .expect("File was opened hence it should have a name");

            // If possible, check if the first row above start row has an ordinal number.
            // If it does, print a warning to the user that their config might be skipping data.
            if row_number > 0 {
                if let Some(row) = range.rows().nth(start_row.saturating_sub(1)) {
                    if let Some(Data::Float(_)) = row.get(0) {
                        log::warn!(
                            "The row before the specified start row ({}) in file: '{}', sheet: '{}' has an ordinal number. \
                            Please check if your config is correct and not skipping any data.",
                            start_row,
                            file_name,
                            sheet_name
                        );
                    }
                }
            }

            let mut transactions = Vec::new();

            // 1. Iterate over the rows, and validate data.
            for row in range.rows().skip(*start_row) {
                // Stop reading when the first date cell is empty.
                if let Some(Data::Empty) = row.get(1) {
                    break;
                }

                let context_message = format!(
                    "File: '{}', Sheet: '{}, Row: {}'",
                    file_name,
                    sheet_name,
                    row_number + 1
                );

                transactions.push(parse_row(row, &context_message).map_err(|message| {
                    format!(
                        "{}: Row {:?}, number {}, has invalid data - please check! Error: {}",
                        context_message, row, row_number, message,
                    )
                })?);

                // Ensure the dates are monotonically increasing.
                if let Some(tx) = transactions.last() {
                    if tx.date() < previous_date {
                        return Err(format!(
                            "{}: Row {:?}, number {}, has a date that is not monotonically increasing - please check!",
                            context_message, row, row_number
                        ).into());
                    }
                    previous_date = tx.date();
                }

                row_number += 1;
            }

            // 2. Ensure this & and a few following cells are actually empty.
            // This is to ensure we don't accidentally skip some data.
            for row in range.rows().skip(row_number).take(3) {
                if row.get(1) != Some(&Data::Empty) {
                    return Err(format!(
                        "Row {:?}, number {} in sheet {}, has non-empty cells after the first empty cell - please check!",
                        row, row_number, sheet_name
                    ).into());
                }

                row_number += 1;
            }

            Ok(transactions)
        } else {
            Err(format!("Sheet '{}' not found", sheet_name).into())
        }
    }
}

impl Iterator for XlsxParser {
    type Item = ParserDataType;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.entries.len() {
            let entry = &self.entries[self.index];
            let result = Self::parse_xlsx_file(entry);
            self.index += 1;

            log::debug!(
                "Parsed transactions from file: {}, sheet: {}",
                entry.file_path,
                entry.sheet_name
            );

            Some(result)
        } else {
            None
        }
    }
}

/// Validate the row data against the expected format, and return the `Transaction`.
/// Each row is validated on its own, without any context of the previous rows.
///
/// # Arguments
/// * `row` - A row of data. Should be in the appropriate format.
/// * `extra_info` - Extra info to attach to the transaction (e.g. filename, sheet, row).
///
/// # Returns
/// * `Transaction` - If the row is valid, return the parsed transaction.
/// * `String` - If the row is invalid, return an error message.
fn parse_row(row: &[Data], extra_info: &str) -> Result<Transaction, String> {
    if row.len() < 8 {
        return Err(format!("Row is too short, skipping: {:?}", row));
    }

    // Helper function to parse a float as a decimal
    fn parse_decimal(data: &Data) -> Result<Decimal, String> {
        if let Data::Float(_) = data {
            Decimal::from_str(
                &data
                    .as_string()
                    .expect("Float can be represented as string."),
            )
            .map_err(|e| format!("Cannot convert float to Decimal: {:?}", e))
        } else {
            Err(format!("Expected a decimal value, found: {:?}", data))
        }
    }

    // Helper function to parse a string
    fn parse_string(data: &Data) -> Result<&str, String> {
        if let Data::String(value) = data {
            Ok(value)
        } else {
            Err(format!("Expected a string, found: {:?}", data))
        }
    }

    // 1. Parse the ordinal value.
    let ordinal = match row[0] {
        Data::Float(value) if value.fract() == 0.0 => value as u32,
        _ => {
            return Err(format!(
                "First column must be an ordinal (integer), skipping: {:?}",
                row
            ))
        }
    };

    // 2. Parse the date.
    let date = match row[1] {
        Data::DateTime(date) => date,
        _ => return Err(format!("Second column must be a date, skipping: {:?}", row)),
    };
    let date = date
        .as_datetime()
        .ok_or_else(|| {
            format!(
                "Cannot convert second column date to `Datetime`, skipping: {:?}",
                row
            )
        })?
        .date();

    // 3. Parse the action type.
    let action_type = if let Data::String(value) = &row[2] {
        TransactionType::from_str(value).map_err(|_| {
            format!(
                "Third column must be a valid action type, skipping: {:?}",
                row
            )
        })?
    } else {
        return Err(format!(
            "Third column must be a string (action type), skipping: {:?}",
            row
        ));
    };

    // 4. Parse the input token.
    let input_token = AssetType::from_str(parse_string(&row[3])?).map_err(|_| {
        format!(
            "Fourth column must be a valid asset type, skipping: {:?}",
            row
        )
    })?;

    // 5. Parse the input amount.
    let input_amount = parse_decimal(&row[4])?;

    // 6. Parse the output token.
    let output_token = AssetType::from_str(parse_string(&row[5])?).map_err(|_| {
        format!(
            "Fifth column must be a valid asset type, skipping: {:?}",
            row
        )
    })?;

    // 7. Parse the output amount.
    let output_amount = parse_decimal(&row[6])?;

    // 8. Parse the note.
    let note = parse_string(&row[7])?;

    // 9. Parse the extra info, if present. Not important.
    let _maybe_extra_info = if let Some(Data::String(value)) = row.get(8) {
        Some(value)
    } else {
        None
    };

    Ok(Transaction::new(
        ordinal,
        date,
        action_type,
        input_token,
        input_amount,
        output_token,
        output_amount,
        note.to_string(),
        extra_info.to_string(),
    ))
}
