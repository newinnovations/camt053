use camt053::Document;

fn main() {
    let camt_file = std::env::args()
        .nth(1)
        .expect("Please provide a CAMT.053 file as an argument");

    let camt = Document::load(&camt_file).expect("Failed to load CAMT.053 file");

    // Dump the parsed CAMT.053 document to the console for inspection
    dbg!(camt);
}
