use camt053::SimpleStatements;

fn main() {
    let camt_file = std::env::args()
        .nth(1)
        .expect("Please provide a CAMT.053 file as an argument");

    let statements = SimpleStatements::load(&camt_file).expect("Failed to load CAMT.053 file");

    for statement in &statements {
        println!("            account: {}", statement.account);
        println!(
            "           currency: {}",
            statement.currency.as_deref().unwrap_or("-")
        );
        println!(
            "    opening balance: {:>10.2} on {}",
            statement.opening_amount, statement.opening_date
        );
        println!(
            "    closing balance: {:>10.2} on {}",
            statement.closing_amount, statement.closing_date
        );
        println!();
        for transaction in statement {
            println!(
                "          reference: {}",
                transaction.reference.as_deref().unwrap_or("-")
            );
            println!("               date: {}", transaction.book_date);
            println!("             amount: {:.2}", transaction.amount);
            println!(
                "  counterparty iban: {}",
                transaction.counter_iban.as_deref().unwrap_or("-")
            );
            println!(
                "               name: {}",
                transaction.counter_name.as_deref().unwrap_or("-")
            );
            println!("        description: {}", transaction.description);
            let clean_desc = transaction.clean_description();
            if clean_desc != transaction.description {
                println!("  clean description: {}", clean_desc);
            }
            println!();
        }
        println!();
    }
}
