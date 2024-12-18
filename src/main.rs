mod fifo;
mod parser;
mod types;
mod validation;

use clap::Parser;
use fifo::InventoryItem;
use types::OutputLine;

use serde::Deserialize;

/// Command-line arguments
#[derive(Debug, Parser)]
struct CmdArgs {
    /// Path to the .toml config file
    #[clap(short, long, default_value = "Config.toml")]
    config_path: String,

    /// Path to the FIFO output file
    #[clap(short, long, default_value = "fifo_output.csv")]
    fifo_output: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // 0. Parse the config file
    let cmd_args = CmdArgs::parse();
    let toml_content = std::fs::read_to_string(cmd_args.config_path)?;
    let config: Config = toml::from_str(&toml_content)?;

    // 1. Parse the XLSX files and validate the data.
    let mut asset_state = Default::default();
    let mut transactions = Vec::new();

    for entry in config.entries {
        let sheet = parser::parse_xlsx_file(&entry.path, &entry.sheet, entry.start_row)?;
        asset_state = validation::validate_sheet(&sheet, asset_state, &entry.sheet)?;

        transactions.extend(sheet.into_iter());
    }

    // 2. Assign ascending ordinals to all transactions.
    let mut counter: u32 = 0;
    transactions = transactions
        .into_iter()
        .map(|tx| {
            counter += 1;
            tx.new_with_ordinal(counter)
        })
        .collect();

    // 3. Create the ledger & process the transactions in FIFO manner.
    let mut ledger = fifo::Ledger::new(transactions);

    println!("{}", ledger.yearly_income_loss_report());

    // 4. Generate the output CSV file.
    let lines = ledger
        .output_lines()
        .into_iter()
        .map(|line| line.to_csv_line(config.csv_delimiter.clone()))
        .collect::<Vec<_>>();

    // Write the output to a file.
    std::fs::write(
        cmd_args.fifo_output,
        format!(
            "{}\n{}",
            OutputLine::csv_header(config.csv_delimiter),
            lines.join("\n")
        ),
    )
    .unwrap();

    Ok(())
}

#[derive(Debug, Deserialize)]
struct Config {
    /// Separator to use in the output CSV file.
    csv_delimiter: String,
    /// List of entries to parse.
    entries: Vec<FileEntry>,
}

#[derive(Debug, Deserialize)]
struct FileEntry {
    /// Path to the XLSX file.
    path: String,
    /// Name of the sheet to read from.
    sheet: String,
    /// Row number from which to start reading the data.
    start_row: usize,
}
