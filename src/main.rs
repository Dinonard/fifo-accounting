mod fifo;
mod price_provider;
mod validation;
mod xlsx_parser;

use fifo_types::{DataParser, MissingPricesCheck, OutputLine};
use price_provider::BasicPriceProvider;
use xlsx_parser::{XlsxFileEntry, XlsxParser};

use clap::Parser;
use env_logger::Env;
use serde::Deserialize;
use std::collections::HashSet;

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
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.level(),
                record.args()
            )
        })
        .init();

    // 0. Parse the config file
    let cmd_args = CmdArgs::parse();
    let toml_content = std::fs::read_to_string(cmd_args.config_path)?;
    let config: Config = toml::from_str(&toml_content)?;

    log::info!("Configuration files loaded successfully.");

    // 1. Parse the XLSX files and validate the data.
    let xlsx_parser = XlsxParser::new(config.entries);
    let transactions = xlsx_parser.parse()?;
    log::info!("Finished parsing all transactions.");

    let final_asset_state = validation::context_validation(&transactions)?;
    log::info!("Contextual validation completed successfully.");
    log::debug!("Final asset state: {:#?}", final_asset_state);

    // Convenience for the user; sanity check.
    let asset_types = transactions
        .iter()
        .map(|tx| vec![tx.output().0.inner(), tx.input().0.inner()])
        .flatten()
        .collect::<HashSet<_>>();
    log::info!("Parsed following unique asset types: {:?}", asset_types);

    // 2. Read the prices from the file.
    let price_provider = BasicPriceProvider::new(&config.price_file)?;
    let missing_prices = price_provider.missing_prices(&transactions);
    if !missing_prices.is_empty() {
        log::error!(
            "Missing prices for the following transactions: {:#?}",
            missing_prices
        );
        return Err("Missing prices for some transactions".into());
    }

    // 3. Create the ledger & process the transactions in FIFO manner.
    let ledger = fifo::Ledger::new(transactions, price_provider);

    log::info!("Yearly income/loss reports:");
    ledger
        .yearly_income_loss_report()
        .iter()
        .for_each(|report| log::info!("{}", report));

    // 4. Generate the output CSV file.
    let lines = ledger
        .output_lines()
        .into_iter()
        .map(|line| line.to_csv_line(config.csv_delimiter.clone()))
        .collect::<Vec<_>>();

    // Write the output to a file.
    std::fs::write(
        &cmd_args.fifo_output,
        format!(
            "{}\n{}",
            OutputLine::csv_header(config.csv_delimiter),
            lines.join("\n")
        ),
    )
    .unwrap();
    log::info!("FIFO breakdown written to file: {}", cmd_args.fifo_output);

    log::info!("Thank you so much for using this program!");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Config {
    /// Separator to use in the output CSV file.
    csv_delimiter: String,
    /// Path to the file with the prices.
    price_file: String,
    /// List of entries to parse.
    entries: Vec<XlsxFileEntry>,
}
