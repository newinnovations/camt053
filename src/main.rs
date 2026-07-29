use camt053::{CamtError, SimpleStatements};
use clap::Parser;
use rust_decimal::Decimal;
use std::{fs::File, io::Write, process::ExitCode};

#[derive(Parser, Debug, Clone)]
#[command(version, about = "CAMT.053 parser and MT940 converter")]
pub struct Args {
    /// CAMT.053 file(s) to open (single .xml file or a .zip containing many .xml files)
    #[arg(value_name = "CAMT.053 FILE(S)", value_hint = clap::ValueHint::FilePath)]
    filenames: Vec<String>,

    /// Show transactions
    #[arg(short, long, default_value_t = false, value_name = "SUMMARY")]
    transactions: bool,

    /// Show reference for each transaction (if available)
    #[arg(short, long, default_value_t = false, value_name = "REFERENCE")]
    r#ref: bool,

    /// Export to MT940 format instead of printing the transactions
    #[arg(short, long, default_value_t = false, value_name = "MT940")]
    mt940: bool,

    /// Combine (export) all statements into a single CAMT.053 file
    #[arg(short, long, value_name = "CAMT.053 FILE")]
    combine: Option<String>,
}

fn run(args: &Args) -> Result<(), CamtError> {
    let mut combined = SimpleStatements::new();
    let mut total_opening_amount = Decimal::ZERO;
    let mut total_closing_amount = Decimal::ZERO;

    if !args.mt940 && !args.transactions {
        println!("{:<38}{:<34}closing balance", "account", "opening balance");
        println!("{}", "-".repeat(92));
    }

    for filename in &args.filenames {
        let statements = SimpleStatements::load(filename)?;
        if args.mt940 {
            for statement in &statements {
                let filename = statement.filename_940();
                let mut f = File::create(&filename)?;
                f.write_all(statement.to_mt940().as_bytes())?;
                println!("Wrote {filename}");
            }
        } else if args.transactions {
            println!();
            println!("CAMT.053 file {}", filename);
            println!();
            for statement in &statements {
                if let Some(currency) = &statement.currency {
                    println!("Account: {} ({currency})", statement.account);
                } else {
                    println!("Account: {}", statement.account);
                }
                println!(
                    "Opening balance: {:>10.2} on {}",
                    statement.opening_amount, statement.opening_date
                );
                println!(
                    "Closing balance: {:>10.2} on {}",
                    statement.closing_amount, statement.closing_date
                );
                if statement.transactions.is_empty() {
                    println!("No transactions");
                } else {
                    println!("Transactions:");
                    for transaction in statement {
                        println!("{transaction}");
                        if args.r#ref {
                            if let Some(reference) = &transaction.reference {
                                println!("{:23}[{reference}]", "");
                            } else {
                                println!("{:23}No reference", "");
                            }
                        }
                    }
                }
                println!();
            }
        } else {
            for statement in &statements {
                total_opening_amount += statement.opening_amount;
                total_closing_amount += statement.closing_amount;
                println!(
                    "{:<30}{:14.2} on {:10}{:6}{:14.2} on {}",
                    statement.account,
                    statement.opening_amount,
                    statement.opening_date,
                    "",
                    statement.closing_amount,
                    statement.closing_date
                );
            }
        }
        combined.extend(statements);
    }

    if !args.mt940 && !args.transactions {
        println!("{}", "-".repeat(92));
        println!(
            "{:<30}{:14.2}{:20}{:14.2}",
            "", total_opening_amount, "", total_closing_amount
        );
    }

    if let Some(combined_filename) = &args.combine {
        let mut f = File::create(combined_filename)?;
        f.write_all(combined.to_camt053()?.as_bytes())?;
        println!("Wrote combined CAMT.053 file {combined_filename}");
    }

    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.filenames.is_empty() {
        println!("Error: please provide at least one CAMT.053 file as an argument");
        return ExitCode::FAILURE;
    }

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}
