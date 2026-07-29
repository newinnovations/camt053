//! Bank/format agnostic representation of a bank statement.
//!
//! The camt.053 XML schema (see [`crate::camt053::schema`]) models the full
//! ISO 20022 message, which is far richer than what this tool needs. This
//! module reduces a parsed [`Document`] down to a small set of structs
//! ([`SimpleStatement`] / [`Transaction`]) holding only the fields required
//! for display or MT940 export: account IBAN, amount, counterparty IBAN,
//! counterparty name and description.

use super::abnamro::abnamro_clean_description;
use crate::{Document, camt053::schema, error::CamtError};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

/// Maximum allowed discrepancy (in the statement's currency) between an
/// opening/closing balance and the sum of transactions (or a neighbouring
/// statement's balance) before it is treated as an inconsistency. Since
/// amounts are exact [`Decimal`] values, this only accounts for genuine
/// rounding quirks in source data, not floating point error.
const BALANCE_TOLERANCE: Decimal = Decimal::from_parts(5, 0, 0, false, 3);

/// A single booked transaction, reduced to the fields required for most use cases.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleTransaction {
    /// IBAN (or other identification) of the account this statement belongs to.
    pub account: String,
    /// Optional reference supplied by the bank for this transaction (may be empty).
    pub reference: Option<String>,
    /// Date the transaction was booked by the bank.
    pub book_date: NaiveDate,
    /// Value date of the transaction (may differ from `book_date`).
    pub value_date: NaiveDate,
    /// Signed amount: negative for debit, positive for credit.
    pub amount: Decimal,
    /// IBAN of the counterparty account, if known.
    pub counter_iban: Option<String>,
    /// Name of the counterparty, if known.
    pub counter_name: Option<String>,
    /// Free-text description of the transaction
    pub description: String,
}

impl SimpleTransaction {
    fn from_entry(account: &str, entry: &schema::Entry) -> Result<Self, CamtError> {
        Ok(SimpleTransaction {
            account: account.to_string(),
            reference: entry
                .reference
                .as_ref()
                .or(entry.account_servicer_reference.as_ref())
                .cloned(),
            book_date: entry.book_date()?,
            value_date: entry.val_date()?,
            amount: entry.amount(),
            counter_iban: entry.counter_iban(),
            counter_name: entry.counter_name(),
            description: entry.description(account)?,
        })
    }

    /// Cleans up the description of a transaction, removing unnecessary whitespace and
    /// replacing SEPA tags with their corresponding values when possible (ABN AMRO).
    pub fn clean_description(&self) -> String {
        let description = if self.account.contains("ABNA") {
            let description = self.description.clone();
            if description.contains("/REMI/") {
                let sepa = super::abnamro::SepaTransaction::parse(&description);
                sepa.remittance_info.unwrap_or(description)
            } else {
                abnamro_clean_description(&self.description)
            }
        } else {
            self.description.clone()
        };
        description.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

impl Display for SimpleTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:>10.2}  {:<22} {:<50}\n{:23}{}",
            self.book_date,
            self.amount,
            self.counter_iban.as_deref().unwrap_or("(no IBAN)"),
            ellipsis_middle(self.counter_name.as_deref().unwrap_or("(no name)"), 50),
            "",
            ellipsis_end(&self.clean_description(), 80),
        )
    }
}

/// A statement: opening/closing balance plus the transactions in between.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleStatement {
    /// IBAN (or other identification) of the account this statement belongs to.
    pub account: String,
    /// Currency of the statement
    pub currency: Option<String>,
    /// Date of the opening balance.
    pub opening_date: NaiveDate,
    /// Opening balance, signed (negative when the account is overdrawn).
    pub opening_amount: Decimal,
    /// Date of the closing balance.
    pub closing_date: NaiveDate,
    /// Closing balance, signed (negative when the account is overdrawn).
    pub closing_amount: Decimal,
    /// Transactions booked between the opening and closing balance.
    pub transactions: Vec<SimpleTransaction>,
}

/// A collection of statements, similar to a CAMT.053 Document.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleStatements {
    /// The statements contained in this collection.
    pub statements: Vec<SimpleStatement>,
}

impl SimpleStatement {
    /// Builds a [`SimpleStatement`] from a raw `schema::Statement`,
    /// validating that the opening/closing balances match the transaction
    /// sum.
    pub fn from_statement(statement: &schema::Statement) -> Result<Self, CamtError> {
        let account = statement.identification()?.to_string();

        let opening = statement.opening_or_closing_previous_day().ok_or_else(|| {
            CamtError::MissingBalance {
                account: account.clone(),
                kind: "opening (or previous day closing)",
            }
        })?;
        let opening_date = opening.date()?;

        let closing = statement
            .closing()
            .ok_or_else(|| CamtError::MissingBalance {
                account: account.clone(),
                kind: "closing",
            })?;
        let closing_date = closing.date()?;

        let transactions: Vec<SimpleTransaction> = statement
            .entry
            .iter()
            .map(|entry| Ok((entry.book_date()?, entry)))
            .collect::<Result<Vec<(NaiveDate, &schema::Entry)>, CamtError>>()?
            .into_iter()
            .filter(|(book_date, _)| *book_date <= closing_date)
            .map(|(_, entry)| SimpleTransaction::from_entry(&account, entry))
            .collect::<Result<_, _>>()?;

        // Check that the opening and closing balances match the transactions, otherwise
        // the statement is inconsistent and we should not silently ignore it.
        let transaction_sum: Decimal = transactions.iter().map(|t| t.amount).sum();
        let expected_closing_amount = opening.amount() + transaction_sum;
        if (expected_closing_amount - closing.amount()).abs() > BALANCE_TOLERANCE {
            return Err(CamtError::InconsistentStatement {
                account: account.clone(),
                opening_date,
                opening_amount: opening.amount(),
                closing_date,
                closing_amount: closing.amount(),
                transaction_sum,
            });
        }

        Ok(SimpleStatement {
            account,
            currency: statement.account.currency.clone(),
            opening_date,
            opening_amount: opening.amount(),
            closing_date,
            closing_amount: closing.amount(),
            transactions,
        })
    }

    /// Converts a full camt.053 [`Document`] into a list of [`SimpleStatement`]s.
    pub fn from_document(doc: &Document) -> Result<Vec<SimpleStatement>, CamtError> {
        doc.statements().iter().map(Self::from_statement).collect()
    }
}

/// Iterator for &SimpleStatement over its contained SimpleTransaction items.
impl<'a> IntoIterator for &'a SimpleStatement {
    type Item = &'a SimpleTransaction;
    type IntoIter = std::slice::Iter<'a, SimpleTransaction>;

    fn into_iter(self) -> Self::IntoIter {
        self.transactions.iter()
    }
}

impl SimpleStatements {
    /// Creates an empty [`SimpleStatements`] collection.
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    /// Appends the statements from another [`SimpleStatements`] collection to this one.
    pub fn extend(&mut self, other: Self) {
        self.statements.extend(other.statements);
    }

    /// Parses the given camt.053 file (plain `.xml` or `.zip` containing several `.xml` files)
    /// and reduces it to the simplified, format agnostic statement structs.
    ///
    /// The format is detected from the file's contents (see [`Self::from_reader`]),
    /// not from the file extension.
    pub fn load<T: AsRef<Path>>(camt_file: T) -> Result<Self, CamtError> {
        let file = File::open(camt_file)?;
        Self::from_reader(BufReader::new(file))
    }

    /// Parses camt.053 data from any reader, transparently handling both a
    /// plain XML document and a `.zip` archive containing several `.xml`
    /// files. The format is detected from the data itself (a leading `PK`
    /// signature indicates a zip archive), so callers don't need to know
    /// the source's format in advance (e.g. an upload without a reliable
    /// filename/extension).
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, CamtError> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        let statements = if data.starts_with(b"PK") {
            let mut archive = zip::ZipArchive::new(Cursor::new(data))?;
            let mut statements = Vec::new();
            for i in 0..archive.len() {
                let file = archive.by_index(i)?;
                if file.name().ends_with(".xml") {
                    let reader = BufReader::new(file);
                    let doc = Document::from_reader(reader)?;
                    statements.extend(Self::vec_from_document(&doc)?);
                }
            }
            statements
        } else {
            let doc = Document::from_reader(data.as_slice())?;
            Self::vec_from_document(&doc)?
        };
        let statements = merge_statements(statements)?;
        Ok(Self { statements })
    }

    /// Converts a full camt.053 [`Document`] into a list of [`SimpleStatement`]s.
    fn vec_from_document(doc: &Document) -> Result<Vec<SimpleStatement>, CamtError> {
        doc.statements()
            .iter()
            .map(SimpleStatement::from_statement)
            .collect()
    }

    /// Converts a full camt.053 [`Document`] into [`SimpleStatements`].
    pub fn from_document(doc: &Document) -> Result<Self, CamtError> {
        Ok(Self {
            statements: Self::vec_from_document(doc)?,
        })
    }
}

/// Implements [`Default`] for [`SimpleStatements`], returning an empty collection.
impl Default for SimpleStatements {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator for &SimpleStatements over its contained SimpleStatement items.
impl<'a> IntoIterator for &'a SimpleStatements {
    type Item = &'a SimpleStatement;
    type IntoIter = std::slice::Iter<'a, SimpleStatement>;

    fn into_iter(self) -> Self::IntoIter {
        self.statements.iter()
    }
}

/// Merges the (typically daily) statements of a zipped camt.053 export
/// into a single [`SimpleStatement`] per account (IBAN).
///
/// Statements for the same account are sorted by opening date and their
/// transactions concatenated in that order. Consecutive statements are
/// required to chain without a gap: the closing balance of one statement
/// must equal the opening balance of the next, and the next statement's
/// opening date may not be before the previous statement's closing date.
/// This assumes the zipped statements together cover a contiguous period
/// (no missing days), which is checked rather than assumed silently: a
/// balance or date mismatch is reported as a [`CamtError`] describing the
/// offending pair.
fn merge_statements(statements: Vec<SimpleStatement>) -> Result<Vec<SimpleStatement>, CamtError> {
    use std::collections::BTreeMap;

    let mut by_identification: BTreeMap<String, Vec<SimpleStatement>> = BTreeMap::new();
    for statement in statements {
        by_identification
            .entry(statement.account.clone())
            .or_default()
            .push(statement);
    }

    by_identification
        .into_values()
        .map(|mut group| {
            group.sort_by_key(|s| s.opening_date);
            // `group` is never empty: it only exists because at least one
            // statement was pushed into it above.
            let account = group[0].account.clone();

            for pair in group.windows(2) {
                let (prev, next) = (&pair[0], &pair[1]);
                if next.opening_date < prev.closing_date {
                    return Err(CamtError::OverlappingStatements {
                        account: account.clone(),
                        prev_closing: prev.closing_date,
                        next_opening: next.opening_date,
                    });
                }
                if (prev.closing_amount - next.opening_amount).abs() > BALANCE_TOLERANCE {
                    return Err(CamtError::BalanceGap {
                        account: account.clone(),
                        prev_closing_date: prev.closing_date,
                        prev_closing_amount: prev.closing_amount,
                        next_opening_date: next.opening_date,
                        next_opening_amount: next.opening_amount,
                    });
                }
                // Check currency consistency across statements for the same account. This is a sanity check, as
                // the camt.053 schema requires all statements for the same account to have the same currency.
                if prev.currency != next.currency {
                    return Err(CamtError::CurrencyMismatch {
                        account: account.clone(),
                        prev_currency: prev.currency.clone(),
                        next_currency: next.currency.clone(),
                    });
                }
            }

            let currency = group[0].currency.clone();

            let opening_date = group.first().expect("group is never empty").opening_date;
            let opening_amount = group.first().expect("group is never empty").opening_amount;
            let closing_date = group.last().expect("group is never empty").closing_date;
            let closing_amount = group.last().expect("group is never empty").closing_amount;

            let mut transactions: Vec<SimpleTransaction> = group
                .into_iter()
                .flat_map(|statement| statement.transactions)
                .collect();
            transactions.sort_by_key(|t| (t.book_date, t.value_date));

            Ok(SimpleStatement {
                account,
                currency,
                opening_date,
                opening_amount,
                closing_date,
                closing_amount,
                transactions,
            })
        })
        .collect()
}

/// Truncates `s` to at most `max_len` characters, replacing the middle
/// section with an ellipsis (`...`) when it is too long. Character based
/// (not byte based), so it is safe to use on any UTF-8 string.
///
/// For example, `ellipsis("hello world", 9)` returns `"hel...rld"`, while
/// `ellipsis("hello world", 20)` returns `"hello world"` unchanged.
pub fn ellipsis_middle(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 3 {
        return "...".chars().take(max_len).collect();
    }

    let keep = max_len - 3;
    let head = keep.div_ceil(2);
    let tail = keep - head;

    let head_str: String = chars[..head].iter().collect();
    let tail_str: String = chars[chars.len() - tail..].iter().collect();
    format!("{head_str}...{tail_str}")
}

/// Truncates `s` to at most `max_len` characters, appending an ellipsis
/// (`...`) when it is too long. Character based (not byte based), so it is
/// safe to use on any UTF-8 string.
pub fn ellipsis_end(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        return s.to_string();
    }
    if max_len == 0 {
        return "".to_string();
    }

    let head_str: String = chars[..max_len].iter().collect();
    format!("{head_str}...")
}

#[cfg(test)]
pub(crate) mod fixtures {
    /// Minimal camt.053 fixture with one debit entry (creditor counterparty
    /// known via `CdtrAcct`) and one credit entry (debtor counterparty known
    /// only by name, no account) plus an `AddtlNtryInf` fallback description.
    ///
    /// Shared with the `mt940` module tests.
    pub const TEST_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document>
  <BkToCstmrStmt>
    <GrpHdr>
      <MsgId>MSG-1</MsgId>
      <CreDtTm>2026-07-22T17:25:22.324+02:00</CreDtTm>
    </GrpHdr>
    <Stmt>
      <Id>0000000000</Id>
      <CreDtTm>2026-07-22T17:25:22.324+02:00</CreDtTm>
      <Acct>
        <Id>
          <IBAN>NL00SNSB0000000000</IBAN>
        </Id>
        <Ccy>EUR</Ccy>
      </Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp>
        <Amt>1000.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2026-01-01</Dt></Dt>
      </Bal>
      <Bal>
        <Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp>
        <Amt>950.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2026-07-21</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt>100.00</Amt>
        <CdtDbtInd>DBIT</CdtDbtInd>
        <Sts>BOOK</Sts>
        <BookgDt><Dt>2026-03-05</Dt></BookgDt>
        <ValDt><Dt>2026-03-05</Dt></ValDt>
        <NtryDtls>
          <TxDtls>
            <RltdPties>
              <Cdtr>
                <Nm>Some Shop B.V.</Nm>
              </Cdtr>
              <CdtrAcct>
                <Id><IBAN>NL00ABNA0000000001</IBAN></Id>
              </CdtrAcct>
            </RltdPties>
            <RmtInf>
              <Ustrd>Payment for invoice 123</Ustrd>
            </RmtInf>
          </TxDtls>
        </NtryDtls>
      </Ntry>
      <Ntry>
        <Amt>50.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Sts>BOOK</Sts>
        <BookgDt><Dt>2026-04-10</Dt></BookgDt>
        <ValDt><Dt>2026-04-10</Dt></ValDt>
        <NtryDtls>
          <TxDtls>
            <RltdPties>
              <Dbtr>
                <Nm>John Doe</Nm>
              </Dbtr>
            </RltdPties>
          </TxDtls>
        </NtryDtls>
        <AddtlNtryInf>Interest payment</AddtlNtryInf>
      </Ntry>
    </Stmt>
  </BkToCstmrStmt>
</Document>"#;
}

#[cfg(test)]
mod tests {
    use super::fixtures::TEST_XML;
    use super::*;
    use crate::Document;
    use rust_decimal_macros::dec;

    fn test_statement() -> SimpleStatement {
        let doc = Document::from_reader(TEST_XML.as_bytes()).expect("valid test fixture");
        SimpleStatement::from_document(&doc)
            .expect("valid statement")
            .remove(0)
    }

    #[test]
    fn ellipsis_keeps_short_strings_untouched() {
        assert_eq!(ellipsis_middle("short", 10), "short");
        assert_eq!(ellipsis_middle("exact", 5), "exact");
    }

    #[test]
    fn ellipsis_truncates_in_the_center() {
        assert_eq!(ellipsis_middle("hello world", 9), "hel...rld");
        assert_eq!(ellipsis_middle("abcdefghij", 7), "ab...ij");
    }

    #[test]
    fn ellipsis_handles_very_small_max_len() {
        assert_eq!(ellipsis_middle("hello", 3), "...");
        assert_eq!(ellipsis_middle("hello", 2), "..");
        assert_eq!(ellipsis_middle("hello", 0), "");
    }

    #[test]
    fn convert_extracts_statement_level_fields() {
        let statement = test_statement();
        assert_eq!(statement.account, "NL00SNSB0000000000");
        assert_eq!(
            statement.opening_date,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
        assert_eq!(statement.opening_amount, dec!(1000.00));
        assert_eq!(
            statement.closing_date,
            NaiveDate::from_ymd_opt(2026, 7, 21).unwrap()
        );
        assert_eq!(statement.closing_amount, dec!(950.00));
        assert_eq!(statement.transactions.len(), 2);
    }

    #[test]
    fn convert_extracts_debit_transaction_with_creditor_counterparty() {
        let statement = test_statement();
        let tx = &statement.transactions[0];
        assert_eq!(tx.account, "NL00SNSB0000000000");
        assert_eq!(tx.amount, dec!(-100.00));
        assert_eq!(tx.counter_iban.as_deref(), Some("NL00ABNA0000000001"));
        assert_eq!(tx.counter_name.as_deref(), Some("Some Shop B.V."));
        assert_eq!(tx.description, "Payment for invoice 123");
    }

    #[test]
    fn convert_extracts_credit_transaction_with_debtor_counterparty() {
        let statement = test_statement();
        let tx = &statement.transactions[1];
        assert_eq!(tx.amount, dec!(50.00));
        assert_eq!(tx.counter_iban, None);
        assert_eq!(tx.counter_name.as_deref(), Some("John Doe"));
        // No <RmtInf><Ustrd> present, but <AddtlNtryInf> is used instead.
        assert_eq!(tx.description, "Interest payment");
    }

    #[test]
    fn display_truncates_long_description() {
        let tx = SimpleTransaction {
            account: "NL00SNSB0000000000".to_string(),
            reference: None,
            book_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            value_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            amount: dec!(12.34),
            counter_iban: Some("NL00ABNA0000000001".to_string()),
            counter_name: Some("Some Shop B.V.".to_string()),
            description: "x".repeat(300),
        };
        let rendered = tx.to_string();
        assert!(rendered.contains("..."));
        assert!(rendered.len() < 300);
    }

    fn statement(
        iban: &str,
        opening_date: &str,
        opening_amount: Decimal,
        closing_date: &str,
        closing_amount: Decimal,
    ) -> SimpleStatement {
        SimpleStatement {
            account: iban.to_string(),
            currency: Some("EUR".to_string()),
            opening_date: NaiveDate::parse_from_str(opening_date, "%Y-%m-%d").unwrap(),
            opening_amount,
            closing_date: NaiveDate::parse_from_str(closing_date, "%Y-%m-%d").unwrap(),
            closing_amount,
            transactions: Vec::new(),
        }
    }

    #[test]
    fn merge_statements_combines_and_sorts_per_account() {
        let s1 = statement(
            "NL00SNSB0000000000",
            "2026-01-02",
            dec!(100.0),
            "2026-01-02",
            dec!(150.0),
        );
        let s2 = statement(
            "NL00SNSB0000000000",
            "2026-01-01",
            dec!(50.0),
            "2026-01-01",
            dec!(100.0),
        );
        let merged = merge_statements(vec![s1, s2]).expect("no overlap or balance gap");

        assert_eq!(merged.len(), 1);
        let m = &merged[0];
        assert_eq!(m.account, "NL00SNSB0000000000");
        assert_eq!(m.opening_date, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(m.opening_amount, dec!(50.0));
        assert_eq!(m.closing_date, NaiveDate::from_ymd_opt(2026, 1, 2).unwrap());
        assert_eq!(m.closing_amount, dec!(150.0));
    }

    #[test]
    fn merge_statements_keeps_accounts_separate() {
        let a = statement(
            "NL00SNSB0000000001",
            "2026-01-01",
            dec!(0.0),
            "2026-01-01",
            dec!(10.0),
        );
        let b = statement(
            "NL00SNSB0000000002",
            "2026-01-01",
            dec!(0.0),
            "2026-01-01",
            dec!(20.0),
        );
        let merged = merge_statements(vec![a, b]).expect("no overlap or balance gap");
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn merge_statements_errors_on_balance_gap() {
        let s1 = statement(
            "NL00SNSB0000000000",
            "2026-01-01",
            dec!(0.0),
            "2026-01-01",
            dec!(100.0),
        );
        let s2 = statement(
            "NL00SNSB0000000000",
            "2026-01-02",
            dec!(999.0),
            "2026-01-02",
            dec!(150.0),
        );
        let err = merge_statements(vec![s1, s2]).unwrap_err();
        assert!(matches!(err, CamtError::BalanceGap { .. }));
        assert!(err.to_string().contains("Balance gap"));
    }

    #[test]
    fn merge_statements_errors_on_overlap() {
        let s1 = statement(
            "NL00SNSB0000000000",
            "2026-01-01",
            dec!(0.0),
            "2026-01-05",
            dec!(100.0),
        );
        let s2 = statement(
            "NL00SNSB0000000000",
            "2026-01-03",
            dec!(100.0),
            "2026-01-06",
            dec!(150.0),
        );
        let err = merge_statements(vec![s1, s2]).unwrap_err();
        assert!(matches!(err, CamtError::OverlappingStatements { .. }));
        assert!(err.to_string().contains("Overlapping"));
    }
}
