# camt053

[![Crates.io](https://img.shields.io/crates/v/camt053.svg)](https://crates.io/crates/camt053)
[![CI](https://github.com/newinnovations/camt053/actions/workflows/ci.yml/badge.svg)](https://github.com/newinnovations/camt053/actions/workflows/ci.yml)
[![Documentation](https://docs.rs/camt053/badge.svg)](https://docs.rs/camt053)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust library and command-line tool for parsing **CAMT.053** (ISO 20022
Bank-to-Customer Statement) files and converting them to **MT940** format.

## Features

- Parse `camt.053.001.02` XML statements (including from `.zip` archives)
- Reduce the verbose CAMT.053 schema into a simple, ergonomic model
  (`SimpleStatement` / `SimpleTransaction`)
- Use this simplified model to import CAMT.053 statements into your own
  applications
- Convert statements to MT940 text format, for use in legacy banking software
- Command-line binary for quick inspection or conversion of statement files

## Installation

As a command-line tool:

```sh
cargo install camt053
```

As a library, add it to your `Cargo.toml`:

```toml
[dependencies]
camt053 = "0.3"
```

The `clap` dependency (used only for the `camt053` command-line binary) is
gated behind the `cli` feature, which is enabled by default. If you only use
the library and want to avoid pulling in `clap`, disable default features:

```toml
[dependencies]
camt053 = { version = "0.3", default-features = false }
```

## Command-line usage

```sh
# Print summary of a CAMT.053 file (or a .zip containing many CAMT.053 files)
camt053 statement.xml

# Print the transactions
camt053 --transactions statement.xml

# Convert to MT940 instead of printing
camt053 --mt940 statement.xml
```

## Library usage

```rust,no_run
use camt053::SimpleStatement;

let statements = SimpleStatement::load("statement.xml")?;
for statement in &statements {
    println!("Account: {}", statement.account);
    for transaction in &statement.transactions {
        println!("{transaction}");
    }
}
# Ok::<(), camt053::CamtError>(())
```

Look at the [examples](examples/) for more usage examples.

## Bank support

Tested with statements from the following banks:

- ABN AMRO
- Rabobank
- SNS Bank (ASN Bank)
- ING Bank

Happy to accept contributions for other banks. Please open an issue or PR if you have a
CAMT.053 file from a bank that is not yet supported.

## License

Licensed under the [MIT license](LICENSE).
