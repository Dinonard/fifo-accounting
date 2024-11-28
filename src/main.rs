mod parser;
mod types;

fn main() {
    println!("Hello, world!");

    parser::read_excel_file("balances.xlsx", "2024", 14).unwrap();
}
