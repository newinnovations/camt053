use camt053::{CamtError, SimpleStatement};
use clap::Parser;
use std::{fs::File, io::Write, process::ExitCode};

#[derive(Parser, Debug, Clone)]
#[command(version, about = "CAMT.053 parser and MT940 converter")]
pub struct Args {
    /// CAMT.053 file to open (single .xml file or a .zip containing many .xml files)
    #[arg(value_name = "FILE OR DIRECTORY", value_hint = clap::ValueHint::FilePath)]
    filename: String,

    /// Export to MT940 format instead of printing the transactions
    #[arg(short, long, default_value_t = false, value_name = "MT940")]
    mt940: bool,
}

fn run(args: &Args) -> Result<(), CamtError> {
    let statements = SimpleStatement::load(&args.filename)?;

    if args.mt940 {
        for statement in &statements {
            let filename = statement.filename_940();
            let mut f = File::create(&filename)?;
            f.write_all(statement.to_mt940().as_bytes())?;
            println!("Wrote {filename}");
        }
    } else {
        for statement in &statements {
            println!("Account: {}", statement.account);
            println!(
                "Opening balance: {:>10.2} on {}",
                statement.opening_amount, statement.opening_date
            );
            println!(
                "Closing balance: {:>10.2} on {}",
                statement.closing_amount, statement.closing_date
            );
            println!("Transactions:");
            for transaction in &statement.transactions {
                println!("{transaction}");
            }
            println!();
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
