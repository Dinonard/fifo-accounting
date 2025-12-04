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

| Ordinal | Date | Transaction Type | Input Token | Input Amount | Output Token | Output Amount |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |

Where:

* `Ordinal` is a simple ordinal number, e.g. **1**, **2**, **3**.
* `Date` is a date in the format on which the transaction happened. E.g. **12-Dec-2024**. Various formats are supported.
* `Transaction Type` is the type of the transaction. E.g. **Buy**, **Sell**, etc.
* `Input Token` is the name (string) of the input type for the transaction. E.g. **BTC**.
* `Input Amount` is the amount of the input token. E.g. **0.14345**.
* `Output Token` is the name (string) of the output type for the transaction. E.g. **EUR**.
* `Output Amount` is the amount of the output token. E.g. **1000.23**.

One example of a transaction:

| Ordinal | Date | Transaction Type | Input Token | Input Amount | Output Token | Output Amount |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 12-Dec-2024 | Swap | ASTR | 10000 | USDT | 644.345 |
| 2 | 12-Dec-2024 | Swap | BTC | 1 | ETH | 25 |

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

## Transaction Types

There are two main transaction types supported:

### Crypto Inflow

* covers buying crypto with fiat currency, receiving interest, airdrops, etc.
* 3 distinct types are supported - `Buying`, `Interest`, `Invoice`
* buy & invoice are treated basically the same, _invoice_ is only used as a type for easier visibility in the input document
* interest is treated separately, and is counted as separate income event
* input currency must be fiat & amount non-zero
* output currency must be crypto & amount non-zero

### Crypto Mutation

* covers selling crypto for fiat currency, and exchanging it for other crypto
* 2 distinct types are supported - `Swap` & `Selling`
* swap requires that non-zero & non-fiat input is exchanged for non-zero & non-fiat output
* selling requires that non-zero & non-fiat input is exchanged for fiat output which can be zero
* selling should be used to cover both _exchange_ for fiat, as well as losses due to fees (hence the output can be zero)

## Note

It is important to note that all fiat amounts **MUST** be expressed in EUR currency.
The program does not support performing conversion, since it requires different exchange rates for different dates.
It is up to the user to ensure that the input data is properly pre-processed to have all fiat amounts in EUR.
