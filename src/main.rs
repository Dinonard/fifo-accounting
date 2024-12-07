mod fifo;
mod parser;
mod types;
mod validation;

fn main() {
    env_logger::init();

    let sheet_2023 = parser::parse_xlsx_file("balances.xlsx", "2023", 1).unwrap();
    let mut result = validation::validate_sheet(&sheet_2023, Default::default(), "2023").unwrap();

    let sheet_2024 = parser::parse_xlsx_file("balances.xlsx", "2024", 14).unwrap();
    result = validation::validate_sheet(&sheet_2024, result, "2024").unwrap();

    println!("Final state: {:#?}", result);
    println!("====================");
}
