mod parser;
mod types;
mod validation;

fn main() {
    env_logger::init();

    parser::parse_xlsx_file("balances.xlsx", "2024", 14).unwrap();
}
