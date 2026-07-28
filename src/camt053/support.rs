use super::schema::{self, Balance, BalanceTypeCode, Document, Entry, Statement};
use crate::error::CamtError;
use chrono::NaiveDate;
use quick_xml::de::Deserializer;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::{fs::File, io::Read, path::Path};

impl Document {
    /// Parses a camt.053 XML file at `source` into the full `Document`.
    ///
    /// This is the full, verbose ISO 20022 representation; most callers should
    /// prefer [`crate::SimpleStatement::load`] instead.
    pub fn load(source: impl AsRef<Path>) -> Result<Self, CamtError> {
        let file = File::open(source)?;
        Self::from_reader(file)
    }

    /// Parses a camt.053 XML document from any buffered reader (e.g. a `.zip`
    /// entry or an in-memory buffer) into the full `Document`.
    ///
    /// On failure, the returned error includes the byte offset and a snippet
    /// of the input surrounding the point where parsing failed.
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, CamtError> {
        // Buffer the whole input so that, on error, we can report a snippet
        // of the surrounding bytes alongside the failure position.
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        let mut deserializer = Deserializer::from_reader(buf.as_slice());
        match Document::deserialize(&mut deserializer) {
            Ok(doc) => Ok(doc),
            Err(source) => {
                let ns_reader = deserializer.get_ref().get_ref();
                // `error_position()` is only set for low-level XML syntax
                // errors; structural/serde errors (e.g. an unexpected tag)
                // leave it at 0, so fall back to `buffer_position()` (the
                // position just after the last event that was read).
                let position = match ns_reader.error_position() {
                    0 => ns_reader.buffer_position(),
                    pos => pos,
                };
                let snippet = error_snippet(&buf, position);
                let (line, column) = line_and_column(&buf, position);
                Err(CamtError::XmlAt {
                    source,
                    position,
                    line,
                    column,
                    snippet,
                })
            }
        }
    }
}

/// Computes the 1-based (line, column) for byte offset `pos` within `data`,
/// counting newlines. `column` is measured in bytes from the start of the
/// line.
fn line_and_column(data: &[u8], pos: u64) -> (usize, usize) {
    let pos = (pos as usize).min(data.len());
    let mut line = 1;
    let mut last_newline = None;
    for (i, &b) in data[..pos].iter().enumerate() {
        if b == b'\n' {
            line += 1;
            last_newline = Some(i);
        }
    }
    let column = match last_newline {
        Some(nl) => pos - nl,
        None => pos + 1,
    };
    (line, column)
}

/// Extracts a human-readable snippet of `data` around byte offset `pos`,
/// clamped to valid UTF-8 boundaries.
fn error_snippet(data: &[u8], pos: u64) -> String {
    const CONTEXT: usize = 40;
    let pos = pos as usize;
    let start = pos.saturating_sub(CONTEXT).min(data.len());
    let end = pos.saturating_add(CONTEXT).min(data.len());

    // Nudge the window to the nearest UTF-8 char boundaries so the lossy
    // conversion below doesn't split a multi-byte character.
    let is_boundary = |i: usize| i == 0 || i == data.len() || (data[i] & 0xC0) != 0x80;
    let start = (start..=pos.min(data.len()))
        .find(|&i| is_boundary(i))
        .unwrap_or(start);
    let end = (end..=data.len()).find(|&i| is_boundary(i)).unwrap_or(end);

    String::from_utf8_lossy(&data[start..end])
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string()
}

impl Statement {
    /// The opening balance (`OPBD`) of this statement, if present.
    pub fn opening(&self) -> Option<&Balance> {
        self.balance.iter().find(|&balance| balance.is_opening())
    }

    /// The closing balance (`CLBD`) of this statement, if present.
    pub fn closing(&self) -> Option<&Balance> {
        self.balance.iter().find(|&balance| balance.is_closing())
    }

    /// The closing balance of the previous day (`PRCD`) of this statement,
    /// if present.
    pub fn closing_previous_day(&self) -> Option<&Balance> {
        self.balance
            .iter()
            .find(|&balance| balance.is_closing_previous_day())
    }

    /// Returns the opening balance if present, otherwise the closing balance of the previous day.
    /// This is useful for statements that don't have an opening balance, but do have a closing balance of the previous day.
    /// In case of a closing balance of the previous day, the opening balance of the current day should match this closing balance.
    /// However the date we return is the date of the opening balance, so we need to correct the date if we return the closing balance of the previous day.
    pub fn opening_or_closing_previous_day(&self) -> Option<Balance> {
        if let Some(opening) = self.opening() {
            return Some(opening.clone());
        }
        let previous_day = self.closing_previous_day()?;
        let opening_date = previous_day.date().ok()?.succ_opt()?;
        let mut opening = previous_day.clone();
        opening.date = schema::DateChoice {
            date: Some(opening_date),
            date_time: None,
        };
        Some(opening)
    }

    /// The account identification: the IBAN if present, otherwise the
    /// `Othr` identification string.
    pub fn identification(&self) -> Result<&str, CamtError> {
        if let Some(iban) = &self.account.identification.iban {
            Ok(iban)
        } else if let Some(other) = &self.account.identification.other {
            Ok(other.identification.as_str())
        } else {
            Err(CamtError::MissingIdentification)
        }
    }
}

impl Balance {
    /// Whether this balance is the opening balance (`OPBD`) of the statement.
    pub fn is_opening(&self) -> bool {
        let matches = matches!(
            self.balance_type.code_or_proprietary.code,
            Some(BalanceTypeCode::OPBD)
        );
        matches
    }

    /// Whether this balance is the closing balance (`CLBD`) of the statement.
    pub fn is_closing(&self) -> bool {
        let matches = matches!(
            self.balance_type.code_or_proprietary.code,
            Some(BalanceTypeCode::CLBD)
        );
        matches
    }

    /// Whether this balance is the closing balance of the previous day
    /// (`PRCD`).
    pub fn is_closing_previous_day(&self) -> bool {
        let matches = matches!(
            self.balance_type.code_or_proprietary.code,
            Some(BalanceTypeCode::PRCD)
        );
        matches
    }

    /// Signed balance amount: negative when `CdtDbtInd` is `DBIT`, positive
    /// when it is `CRDT`.
    pub fn amount(&self) -> Decimal {
        match self.credit_debit_indicator {
            schema::CreditDebitIndicator::DBIT => -self.amount,
            schema::CreditDebitIndicator::CRDT => self.amount,
        }
    }

    /// The date of this balance.
    pub fn date(&self) -> Result<NaiveDate, CamtError> {
        self.date.date()
    }
}

impl Entry {
    /// Signed transaction amount: negative when `CdtDbtInd` is `DBIT`,
    /// positive when it is `CRDT`.
    pub fn amount(&self) -> Decimal {
        match self.credit_debit_indicator {
            schema::CreditDebitIndicator::DBIT => -self.amount,
            schema::CreditDebitIndicator::CRDT => self.amount,
        }
    }

    /// The value date of this entry.
    pub fn val_date(&self) -> Result<NaiveDate, CamtError> {
        self.value_date
            .as_ref()
            .ok_or(CamtError::MissingDate)?
            .date()
    }

    /// The booking date of this entry.
    pub fn book_date(&self) -> Result<NaiveDate, CamtError> {
        self.booking_date
            .as_ref()
            .ok_or(CamtError::MissingDate)?
            .date()
    }

    /// The single `RltdPties` block of this entry's (sole) transaction
    /// detail, if present.
    fn related_parties(&self) -> Option<&schema::RelatedParties> {
        let details = self.details.first()?;
        let details = details.transaction_details.first()?;
        details.related_parties.as_ref()
    }

    /// Name of the counterparty: the debtor for an incoming (credit) entry,
    /// the creditor for an outgoing (debit) entry.
    pub fn counter_name(&self) -> Option<String> {
        let related_parties = self.related_parties()?;
        let party = match self.credit_debit_indicator {
            schema::CreditDebitIndicator::CRDT => related_parties.debtor.as_ref(),
            schema::CreditDebitIndicator::DBIT => related_parties.creditor.as_ref(),
        };
        party?.effective().name.clone()
    }

    /// IBAN of the counterparty account: the debtor account for an incoming
    /// (credit) entry, the creditor account for an outgoing (debit) entry.
    pub fn counter_iban(&self) -> Option<String> {
        let related_parties = self.related_parties()?;
        let account = match self.credit_debit_indicator {
            schema::CreditDebitIndicator::CRDT => related_parties.debtor_account.as_ref(),
            schema::CreditDebitIndicator::DBIT => related_parties.creditor_account.as_ref(),
        };
        account?.identification.iban.clone()
    }

    /// The transaction description, falling back from `AddtlNtryInf` to the
    /// (sole) transaction detail's `RmtInf`/`Ustrd`. `account` is used
    /// only to identify the offending entry in error messages.
    pub fn description(&self, account: &str) -> Result<String, CamtError> {
        if let Some(addtl_ntry_inf) = &self.additional_information {
            Ok(addtl_ntry_inf.clone())
        } else {
            let details =
                self.details
                    .first()
                    .ok_or_else(|| CamtError::MissingTransactionDetails {
                        account: account.to_string(),
                    })?;
            let details = &details.transaction_details;
            if details.len() != 1 {
                return Err(CamtError::UnexpectedTransactionDetailsCount {
                    account: account.to_string(),
                    count: details.len(),
                });
            }
            let rmtinf = details
                .first()
                .ok_or_else(|| CamtError::MissingTransactionDetails {
                    account: account.to_string(),
                })?
                .remittance_information
                .as_ref();
            if let Some(rmtinf) = rmtinf.and_then(|rmtinf| rmtinf.unstructured.first()) {
                Ok(rmtinf.clone())
            } else {
                Ok(Default::default())
            }
        }
    }
}

impl Document {
    /// The statements (`<Stmt>` elements) contained in this document.
    pub fn statements(&self) -> &Vec<Statement> {
        &self.bank_to_customer_statement.statements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn import(balances_xml: &str) -> Document {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
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
      </Acct>
      {balances_xml}
    </Stmt>
  </BkToCstmrStmt>
</Document>"#
        );
        Document::from_reader(xml.as_bytes()).expect("valid test fixture")
    }

    #[test]
    fn from_reader_reports_position_and_snippet_on_malformed_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document>
  <BkToCstmrStmt>
    <GrpHdr>
      <MsgId>MSG-1</MsgId>
      <CreDtTm>2026-07-22T17:25:22.324+02:00</CreDtTm>
    </GrpHdr>
    <Stmt>
      <Id>0000000000</Id>
      <CreDtTm>2026-07-22T17:25:22.324+02:00
      <Acct>
        <Id>
          <IBAN>NL00SNSB0000000000</IBAN>
        </Id>
      </Acct>
    </Stmt>
  </BkToCstmrStmt>
</Document>"#;

        let err = Document::from_reader(xml.as_bytes()).expect_err("malformed XML must fail");
        match err {
            CamtError::XmlAt {
                position,
                line,
                column,
                snippet,
                ..
            } => {
                assert!(position > 0, "expected a non-zero error position");
                assert!(
                    line > 1,
                    "expected a line number past the first line, got {line}"
                );
                assert!(column > 0, "expected a non-zero column, got {column}");
                assert!(
                    snippet.contains("CreDtTm") || snippet.contains("Acct"),
                    "snippet should contain context around the error, got: {snippet:?}"
                );
            }
            other => panic!("expected CamtError::XmlAt, got: {other:?}"),
        }
    }

    #[test]
    fn opening_or_closing_previous_day_prefers_opening_balance() {
        let doc = import(
            r#"<Bal>
                <Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp>
                <Amt>1000.00</Amt>
                <CdtDbtInd>CRDT</CdtDbtInd>
                <Dt><Dt>2026-01-02</Dt></Dt>
              </Bal>
              <Bal>
                <Tp><CdOrPrtry><Cd>PRCD</Cd></CdOrPrtry></Tp>
                <Amt>900.00</Amt>
                <CdtDbtInd>CRDT</CdtDbtInd>
                <Dt><Dt>2026-01-01</Dt></Dt>
              </Bal>"#,
        );
        let statement = &doc.statements()[0];
        let balance = statement
            .opening_or_closing_previous_day()
            .expect("balance present");
        assert_eq!(
            balance.date().unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 2).unwrap()
        );
        assert_eq!(balance.amount(), dec!(1000.00));
    }

    #[test]
    fn opening_or_closing_previous_day_falls_back_and_corrects_date() {
        let doc = import(
            r#"<Bal>
                <Tp><CdOrPrtry><Cd>PRCD</Cd></CdOrPrtry></Tp>
                <Amt>900.00</Amt>
                <CdtDbtInd>CRDT</CdtDbtInd>
                <Dt><Dt>2026-01-01</Dt></Dt>
              </Bal>"#,
        );
        let statement = &doc.statements()[0];
        let balance = statement
            .opening_or_closing_previous_day()
            .expect("balance present");
        // The PRCD balance is dated the previous day; the effective
        // opening date is the day after.
        assert_eq!(
            balance.date().unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 2).unwrap()
        );
        assert_eq!(balance.amount(), dec!(900.00));
    }

    #[test]
    fn opening_or_closing_previous_day_is_none_when_neither_present() {
        let doc = import(
            r#"<Bal>
                <Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp>
                <Amt>900.00</Amt>
                <CdtDbtInd>CRDT</CdtDbtInd>
                <Dt><Dt>2026-01-01</Dt></Dt>
              </Bal>"#,
        );
        let statement = &doc.statements()[0];
        assert!(statement.opening_or_closing_previous_day().is_none());
    }
}
