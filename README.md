# FIFO accounting

## Description

Simple FIFO accounting system to be used for easy crypto currency accounting.
The system takes a list of files with transactions as the input, and produces a FIFO accounting report as the output.

**NOTE: This is still under development.**

## Usage

Configure the `Config.toml` file:

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

Files & sheets can be provided in any order.
The transactions must be sequential per file (monotonically increasing date), and it's not allowed to go into negative balance (e.g. user cannot have **-0.01 BTC** at any point).

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

## Expected XMLX Format

Expected format is:

| Ordinal | Date | Transaction Type | Input Token | Input Amount | Output Token | Output Amount | Note | Additional Info |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |

Where:

* `Ordinal` is a simple ordinal number, e.g. **1**, **2**, **3**.
* `Date` is a date in the format on which the transaction happened. E.g. **12-Dec-2024**. Various formats are supported.
* `Transaction Type` is the type of the transaction. E.g. **Buy**, **Sell**, etc.
* `Input Token` is the name (string) of the input type for the transaction. E.g. **BTC**.
* `Input Amount` is the amount of the input token. E.g. **0.14345**.
* `Output Token` is the name (string) of the output type for the transaction. E.g. **EUR**.
* `Output Amount` is the amount of the output token. E.g. **1000.23**.
* `Note` is a note for the transaction. E.g. **DEX Swap**. It's free form text, irrelevant for this program.
* `Additional Info` is additional free form text, used to provide more extensive information about the transaction. E.g. link to the transaction on the block explorer.

For the list of valid `Transaction Types`, please refer to the code.

One example of a transaction:

| Ordinal | Date | Transaction Type | Input Token | Input Amount | Output Token | Output Amount | Note | Additional Info |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 12-Dec-2024 | Swap | ASTR | 10000 | USDT | 644.345 | DEX Swap | _link to the transaction on block explorer_ |
| 2 | 12-Dec-2024 | Swap | BTC | 1 | ETH | 25 | Binance |  |

## Custom Data Parser

It is possible to easily modify the program to support different data formats by adding a custom parser.

To do so, user should only implement the following iterator trait like:

```Rust

use types::{ParserDataType, Transaction};

pub struct CustomParser;

impl Iterator<Item = ParserDataType> for CustomParser {
    fn next(&mut self) -> Option<ParserDataType> {
        // Implement the custom parser here
    }
}
```

This _custom parser_ can then be turned into the `TransactionsProvider` in the `main.rs` file:

```Rust
let tx_provider: TransactionProvider<_> = CustomParser::new(...).into();
```

Rest of the pipeline remains the same and can be reused.
