// TODO: make it generic later, this is just to get some data into the program
// Ideally it will be configurable via a static config file, to speed up further usage.

use calamine::{open_workbook, Data, Reader, Xlsx};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::validation::parse_row;
use fifo_types::{ParserDataType, Transaction};

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
            let context_message = format!("File: '{}', Sheet: '{}'", file_name, row_number);

            let mut transactions = Vec::new();

            // 1. Iterate over the rows, and validate data.
            for row in range.rows().skip(*start_row) {
                // Stop reading when the first date cell is empty.
                if let Some(Data::Empty) = row.get(1) {
                    break;
                }

                transactions.push(parse_row(row).map_err(|message| {
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
