mod parser;
mod types;

fn main() {
    env_logger::init();

    parser::read_excel_file("balances.xlsx", "2024", 14).unwrap();
}
