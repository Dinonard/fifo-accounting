# FIFO accounting

## Description

Simple FIFO accounting system to be used for easy crypto currency accounting.
The system takes a list of files with transactions as the input, and produces a FIFO accounting report as the output.

**NOTE: This is still under development, and not everything is correct yet.**

## Usage

Configure the `.toml` file:

```toml
csv_delimiter = ";"

[[entries]]
path = "balances.xlsx"
sheet = "2023"
start_row = 1

[[entries]]
path = "balances.xlsx"
sheet = "2024"
start_row = 14
```

Sheets should be provided in the order they should be processed.
This might change in the future.

Check the binary options:

```bash
Command-line arguments

Usage: fifo-accounting [OPTIONS]

Options:
  -c, --config-path <CONFIG_PATH>  Path to the .toml config file [default: Config.toml]
  -f, --fifo-output <FIFO_OUTPUT>  Path to the FIFO output file [default: fifo_output.csv]
  -h, --help                       Print help
  ```

Run the binary:

```bash
cargo run -- -c Config.toml -f fifo_output.csv
```
