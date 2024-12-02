// TODO: make it generic later, this is just to get some data into the program
// Ideally it will be configurable via a static config file, to speed up further usage.

use calamine::{open_workbook, Data, DataType, Reader, Xlsx};

pub fn read_excel_file(
    file_path: &str,
    sheet_name: &str,
    start_row: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    if let Ok(range) = workbook.worksheet_range(sheet_name) {
        let mut row_number = start_row;

        // 1. Iterate over the rows, and validate data.
        for row in range.rows().skip(start_row) {
            // Stop reading when the first date cell is empty
            // TODO: improve this since we might accidentally miss some data.
            if let Some(Data::Empty) = row.get(1) {
                break;
            }

            if !validate_row(&row) {
                println!("Row {} is invalid.", row_number);
            }

            row_number += 1;
        }

        // 2. Ensure this & following cells are actually empty. This is to ensure we don't accidentally skip some data.
        for row in range.rows().skip(row_number).take(10) {
            if row.get(1) != Some(&Data::Empty) {
                log::error!(
                    "Row {:?}, number {}, has non-empty cells after the first empty cell - please check!",
                    row, row_number,
                );
                break;
            }

            row_number += 1;
        }
    } else {
        log::error!("Sheet '{}' not found", sheet_name);
    }

    Ok(())
}

fn validate_row(row: &[Data]) -> bool {
    // TODO: add row metadata for easier debugging (file name, sheet, row number, etc.)
    // TODO2: function should return a parsed struct, not just boolean
    if row.len() < 9 {
        log::error!("Row is too short, skipping: {:?}", row);
        return false;
    }

    // TODO: verify it's an integer
    if !row[0].is_float() {
        log::error!(
            "First column must be an ordinal (integer), skipping: {:?}",
            row
        );
        return false;
    }

    if !row[1].is_datetime() {
        log::error!("Second column must be a date, skipping: {:?}", row);
        return false;
    }

    // TODO: introduce custom type & check against it
    if !row[2].is_string() {
        log::error!(
            "Third column must be a string (action type), skipping: {:?}",
            row
        );
        return false;
    }

    // TODO: introduce custom type & config file with allowed values
    if !row[3].is_string() {
        log::error!(
            "Fourth column must be a string (token name), skipping: {:?}",
            row
        );
        return false;
    }

    if !row[4].is_float() {
        log::error!(
            "Fifth column must be a decimal (token amount), skipping: {:?}",
            row
        );
        return false;
    }

    if !row[5].is_string() {
        log::error!(
            "Sixth column must be a string (token name), skipping: {:?}",
            row
        );
        return false;
    }

    if !row[6].is_float() {
        log::error!(
            "Seventh column must be a decimal (token amount), skipping: {:?}",
            row
        );
        return false;
    }

    if !row[7].is_string() {
        log::error!("Eighth column must be a string (note), skipping: {:?}", row);
        return false;
    }

    if !row[8].is_string() && !row[8].is_empty() {
        log::error!(
            "Ninth column, if present, must be a string (extra info), skipping: {:?}",
            row
        );
        return false;
    }

    true
}
