use camt053::{CamtError, SimpleStatements};
use clap::Parser;
use std::{fs::File, io::Write, process::ExitCode};

#[derive(Parser, Debug, Clone)]
#[command(version, about = "CAMT.053 parser and MT940 converter")]
pub struct Args {
    /// CAMT.053 file to open (single .xml file or a .zip containing many .xml files)
    #[arg(value_name = "FILE OR DIRECTORY", value_hint = clap::ValueHint::FilePath)]
    filename: String,

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
    let statements = SimpleStatements::load(&args.filename)?;

    if args.mt940 {
        for statement in &statements {
            let filename = statement.filename_940();
            let mut f = File::create(&filename)?;
            f.write_all(statement.to_mt940().as_bytes())?;
            println!("Wrote {filename}");
        }
    } else {
        println!();
        println!("CAMT.053 file {}", args.filename);
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
            if args.transactions {
                if statement.transactions.is_empty() {
                    println!("No transactions");
                } else {
                    println!("Transactions:");
                    for transaction in &statement.transactions {
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
            }
            println!();
        }

        if let Some(combined_filename) = &args.combine {
            let mut f = File::create(combined_filename)?;
            f.write_all(statements.to_camt053()?.as_bytes())?;
            println!("Wrote combined CAMT.053 file {combined_filename}");
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}
