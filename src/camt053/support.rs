use super::schema::{self, Balance, BalanceTypeCode, Document, Entry, Statement};
use crate::error::CamtError;
use chrono::NaiveDate;
use quick_xml::de::from_reader;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

impl Document {
    /// Parses a camt.053 XML file at `source` into the full `Document`.
    ///
    /// This is the full, verbose ISO 20022 representation; most callers should
    /// prefer [`crate::SimpleStatement::load`] instead.
    pub fn load(source: impl AsRef<Path>) -> Result<Self, CamtError> {
        let file = File::open(source)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Parses a camt.053 XML document from any buffered reader (e.g. a `.zip`
    /// entry or an in-memory buffer) into the full `Document`.
    pub fn from_reader<R: BufRead + Read>(reader: R) -> Result<Self, CamtError> {
        let doc = from_reader(reader)?;
        Ok(doc)
    }
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
    pub fn amount(&self) -> f64 {
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
    pub fn amount(&self) -> f64 {
        match self.credit_debit_indicator {
            schema::CreditDebitIndicator::DBIT => -self.amount,
            schema::CreditDebitIndicator::CRDT => self.amount,
        }
    }

    /// The value date of this entry.
    pub fn val_date(&self) -> Result<NaiveDate, CamtError> {
        self.value_date.date()
    }

    /// The booking date of this entry.
    pub fn book_date(&self) -> Result<NaiveDate, CamtError> {
        self.booking_date.date()
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
        party?.name.clone()
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
                Ok("no details".to_string())
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
        assert_eq!(balance.amount(), 1000.00);
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
        assert_eq!(balance.amount(), 900.00);
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
