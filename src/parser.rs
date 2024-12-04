// TODO: make it generic later, this is just to get some data into the program
// Ideally it will be configurable via a static config file, to speed up further usage.

use calamine::{open_workbook, Data, Reader, Xlsx};

use crate::{types::Transaction, validation::parse_row};

/// Parse the XLSX file and return the transactions from the specified sheet.
/// Performs basic validation of each row data, and fails if any row is invalid.
///
/// Parsing starts from the specified row number, and stops when the first empty _date_ cell is found.
///
/// # Arguments
/// * `file_path` - Path to the XLSX file.
/// * `sheet_name` - Name of the sheet to read from.
/// * `start_row` - Row number from which to start reading the data.
///
pub fn parse_xlsx_file(
    file_path: &str,
    sheet_name: &str,
    start_row: usize,
) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    if let Ok(range) = workbook.worksheet_range(sheet_name) {
        let mut row_number = start_row;

        let file_name = file_path
            .split('/')
            .last()
            .expect("File was opened hence it should have a name");
        let context_message = format!("File: '{}', Sheet: '{}'", file_name, row_number);

        let mut transactions = Vec::new();

        // TODO: add a wrapper around row so it can be formatted more nicely?

        // 1. Iterate over the rows, and validate data.
        for row in range.rows().skip(start_row) {
            // Stop reading when the first date cell is empty.
            if let Some(Data::Empty) = row.get(1) {
                break;
            }

            transactions.push(parse_row(row).map_err(|message| {
                format!(
                    "Row {:?}, number {}, has invalid data - please check! Error: {}",
                    row, row_number, message,
                )
            })?);

            row_number += 1;
        }

        // 2. Ensure this & and a few following cells are actually empty.
        // This is to ensure we don't accidentally skip some data.
        for row in range.rows().skip(row_number).take(3) {
            if row.get(1) != Some(&Data::Empty) {
                return Err(format!(
                    "Row {:?}, number {}, has non-empty cells after the first empty cell - please check!",
                    row, row_number,
                ).into());
            }

            row_number += 1;
        }

        Ok(transactions)
    } else {
        Err(format!("Sheet '{}' not found", sheet_name).into())
    }
}
