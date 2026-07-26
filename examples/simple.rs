use camt053::SimpleStatement;

fn main() {
    let camt_file = std::env::args()
        .nth(1)
        .expect("Please provide a CAMT.053 file as an argument");

    let statements = SimpleStatement::load(&camt_file).expect("Failed to load CAMT.053 file");

    statements.into_iter().for_each(|statement| {
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
            println!();
        }
        println!();
    });
}
