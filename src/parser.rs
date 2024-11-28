// TODO: make it generic later, this is just to get some data into the program
// Ideally it will be configurable via a static config file, to speed up further usage.

use calamine::{open_workbook, Data, Reader, Xlsx};

pub fn read_excel_file(
    file_path: &str,
    sheet_name: &str,
    start_row: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Open the Excel file
    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    // Read the specified sheet
    if let Ok(range) = workbook.worksheet_range(sheet_name) {
        for row in range.rows().skip(start_row) {
            for cell in row {
                print!("{:?}\t", cell);
            }
            println!();

            // If we have both input & output as "EMPTY", we can stop reading the file
            println!("DATA 3: {:?}", row.get(3).unwrap());
            if let (Some(Data::String(t1)), Some(Data::String(t2))) = (row.get(3), row.get(5))
            {
                if t1 == "EMPTY" && t2 == "EMPTY" {
                    break;
                }
            }
        }
    } else {
        println!("Sheet '{}' not found", sheet_name);
    }

    Ok(())
}
