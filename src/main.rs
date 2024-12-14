mod fifo;
mod parser;
mod types;
mod validation;

use types::OutputLine;

fn main() {
    env_logger::init();

    let sheet_2023 = parser::parse_xlsx_file("balances.xlsx", "2023", 1).unwrap();
    let mut result = validation::validate_sheet(&sheet_2023, Default::default(), "2023").unwrap();

    let sheet_2024 = parser::parse_xlsx_file("balances.xlsx", "2024", 14).unwrap();
    result = validation::validate_sheet(&sheet_2024, result, "2024").unwrap();

    println!("Final state: {:#?}", result);
    println!("====================");

    let mut counter: u32 = 0;
    let transactions: Vec<_> = sheet_2023
        .into_iter()
        .chain(sheet_2024.into_iter())
        .map(|tx| {
            counter += 1;
            tx.new_with_ordinal(counter)
        })
        .collect();

    let mut ledger = fifo::Ledger::new(transactions);

    let lines = ledger
        .output_lines()
        .into_iter()
        .map(|line| line.to_csv_line(";".to_string()))
        .collect::<Vec<_>>();

    // Write the output to a file.
    std::fs::write(
        "output.csv",
        format!(
            "{}\n{}",
            OutputLine::csv_header(";".to_string()),
            lines.join("\n")
        ),
    )
    .unwrap();

    // ledger.in_order()
    //     .iter()
    //     .for_each(|tx| {
    //         println!("{}", tx.report());
    //     });

    // println!("State after processing:");
    // println!("{}", ledger.remaining_amount_report());

    // ledger.ledger()[&crate::types::AssetType::EUR]
    //     .iter()
    //     .for_each(|tx| {
    //         println!("{:?}", tx);
    //     });
}
