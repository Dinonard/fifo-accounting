// TODO: make it generic later, this is just to get some data into the program
// Ideally it will be configurable via a static config file, to speed up further usage.

use calamine::{open_workbook, Data, DataType, Reader, Xlsx};
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::types::{Transaction, TransactionType};

pub fn read_excel_file(
    file_path: &str,
    sheet_name: &str,
    start_row: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    if let Ok(range) = workbook.worksheet_range(sheet_name) {
        let mut row_number = start_row;

        let file_name = file_path
            .split('/')
            .last()
            .expect("File was opened hence it should have a name");
        let context_message = format!("File: '{}', Sheet: '{}'", file_name, row_number);

        // 1. Iterate over the rows, and validate data.
        for row in range.rows().skip(start_row) {
            // Stop reading when the first date cell is empty.
            if let Some(Data::Empty) = row.get(1) {
                break;
            }

            if let Err(message) = validate_row(&row, &context_message) {
                log::error!("{}", message);
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

fn validate_row(row: &[Data], context: &str) -> Result<Transaction, String> {
    if row.len() < 9 {
        return Err(format!(
            "{}, Row is too short, skipping: {:?}",
            context, row
        ));
    }

    // 1. Parse the ordinal value.
    let ordinal = if let Data::Float(value) = row[0] {
        value
    } else {
        return Err(format!(
            "{}, First column must be an ordinal (integer), skipping: {:?}",
            context, row
        ));
    };
    if ordinal.fract() != 0.0 {
        return Err(format!(
            "{}, First column must be an ordinal (integer), skipping: {:?}",
            context, row
        ));
    }
    let ordinal = ordinal as u32;

    // 2. Parse the date.
    let date = if let Data::DateTime(date) = row[1] {
        date
    } else {
        return Err(format!(
            "{}, Second column must be a date, skipping: {:?}",
            context, row
        ));
    };
    let date = if let Some(date) = date.as_datetime() {
        date
    } else {
        return Err(format!(
            "{}, Cannot convert second column date to `Datetime`, skipping: {:?}",
            context, row
        ));
    };

    // 3. Parse the action type.
    let action_type = if let Data::String(value) = &row[2] {
        TransactionType::from_str(value).map_err(|_| {
            format!(
                "{}, Third column must be a valid action type, skipping: {:?}",
                context, row
            )
        })?
    } else {
        return Err(format!(
            "{}, Third column must be a string (action type), skipping: {:?}",
            context, row
        ));
    };

    // 4. Parse the input token.
    // TODO: introduce custom type & config file with allowed values
    let input_token = if let Data::String(value) = &row[3] {
        value
    } else {
        return Err(format!(
            "{}, Fourth column must be a string (token name), skipping: {:?}",
            context, row
        ));
    };

    // 5. Parse the input amount.
    let input_amount = if let Data::Float(_) = row[4] {
        Decimal::from_str(
            &row[4]
                .as_string()
                .expect("Float can be converted to string"),
        )
        .expect("It's string representation of a float")
    } else {
        return Err(format!(
            "{}, Fifth column must be a decimal (token amount), skipping: {:?}",
            context, row
        ));
    };

    // 6. Parse the output token.
    let output_token = if let Data::String(value) = &row[5] {
        value
    } else {
        return Err(format!(
            "{}, Sixth column must be a string (token name), skipping: {:?}",
            context, row
        ));
    };

    // 7. Parse the output amount.
    let output_amount = if let Data::Float(_) = row[6] {
        Decimal::from_str(
            &row[6]
                .as_string()
                .expect("Float can be converted to string"),
        )
        .expect("It's string representation of a float")
    } else {
        return Err(format!(
            "{}, Seventh column must be a decimal (token amount), skipping: {:?}",
            context, row
        ));
    };

    // 8. Parse the note.
    let note = if let Data::String(value) = &row[7] {
        value
    } else {
        return Err(format!(
            "{}, Eighth column must be a string (note), skipping: {:?}",
            context, row
        ));
    };

    // 9. Parse the extra info, if present. Not important.
    let _maybe_extra_info = if let Data::String(value) = &row[8] {
        Some(value)
    } else {
        None
    };

    Ok(Transaction::new(
        ordinal,
        date,
        action_type,
        input_token.to_string(),
        input_amount,
        output_token.to_string(),
        output_amount,
        note.to_string(),
    ))
}
