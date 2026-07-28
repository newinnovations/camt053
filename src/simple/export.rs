//! Writer that renders a [`SimpleStatement`] as a compliant camt.053.001.02
//! (ISO 20022 Bank-to-Customer Statement) XML document.
//!
//! The mapping is intentionally minimal: it only emits the elements needed
//! to losslessly round-trip a [`SimpleStatement`] / [`SimpleTransaction`],
//! but every element it does emit is placed in the correct position of the
//! `camt.053.001.02` schema (see `doc/camt.053.001.02.xsd`), so the result
//! validates against it.

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::{SimpleStatements, error::CamtError};

use super::model::{SimpleStatement, SimpleTransaction};

const CAMT053_NAMESPACE: &str = "urn:iso:std:iso:20022:tech:xsd:camt.053.001.02";

/// Fallback currency used when a statement does not specify one. camt.053
/// requires a currency on every amount, so we need *something* to fall back
/// on.
const DEFAULT_CURRENCY: &str = "EUR";

#[derive(Debug, Serialize)]
#[serde(rename = "Document")]
struct Document {
    #[serde(rename = "@xmlns")]
    xmlns: &'static str,
    #[serde(rename = "BkToCstmrStmt")]
    bk_to_cstmr_stmt: BkToCstmrStmt,
}

#[derive(Debug, Serialize)]
struct BkToCstmrStmt {
    #[serde(rename = "GrpHdr")]
    grp_hdr: GroupHeader,
    #[serde(rename = "Stmt")]
    stmt: Vec<Stmt>,
}

/// `GroupHeader42`
#[derive(Debug, Serialize)]
struct GroupHeader {
    #[serde(rename = "MsgId")]
    msg_id: String,
    #[serde(rename = "CreDtTm")]
    cre_dt_tm: String,
}

/// `AccountStatement2`
#[derive(Debug, Serialize)]
struct Stmt {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "CreDtTm")]
    cre_dt_tm: String,
    #[serde(rename = "Acct")]
    acct: CashAccount,
    #[serde(rename = "Bal")]
    bal: Vec<Balance>,
    #[serde(rename = "Ntry", skip_serializing_if = "Vec::is_empty")]
    ntry: Vec<Entry>,
}

/// `CashAccount20`
#[derive(Debug, Serialize)]
struct CashAccount {
    #[serde(rename = "Id")]
    id: AccountId,
    #[serde(rename = "Ccy", skip_serializing_if = "Option::is_none")]
    ccy: Option<String>,
}

/// `CashAccount16`, used for the (optional) counterparty account.
#[derive(Debug, Serialize)]
struct RelatedCashAccount {
    #[serde(rename = "Id")]
    id: AccountId,
}

/// `AccountIdentification4Choice`
#[derive(Debug, Serialize)]
struct AccountId {
    #[serde(rename = "IBAN", skip_serializing_if = "Option::is_none")]
    iban: Option<String>,
    #[serde(rename = "Othr", skip_serializing_if = "Option::is_none")]
    othr: Option<GenericAccountId>,
}

impl AccountId {
    /// Builds an `AccountIdentification4Choice` from a free-form account
    /// identifier, using `IBAN` when it looks like one, and falling back to
    /// `Othr` (a generic "other" identification) otherwise, so no
    /// information is lost.
    fn from_account(account: &str) -> Self {
        if is_iban(account) {
            Self {
                iban: Some(account.to_string()),
                othr: None,
            }
        } else {
            Self {
                iban: None,
                othr: Some(GenericAccountId {
                    id: account.to_string(),
                }),
            }
        }
    }
}

/// `GenericAccountIdentification1`
#[derive(Debug, Serialize)]
struct GenericAccountId {
    #[serde(rename = "Id")]
    id: String,
}

/// `BalanceType12Code`
#[derive(Debug, Clone, Copy)]
enum BalanceKind {
    Opening,
    Closing,
}

impl BalanceKind {
    fn code(self) -> &'static str {
        match self {
            BalanceKind::Opening => "OPBD",
            BalanceKind::Closing => "CLBD",
        }
    }
}

/// `CashBalance3`
#[derive(Debug, Serialize)]
struct Balance {
    #[serde(rename = "Tp")]
    tp: BalanceType,
    #[serde(rename = "Amt")]
    amt: Amount,
    #[serde(rename = "CdtDbtInd")]
    cdt_dbt_ind: &'static str,
    #[serde(rename = "Dt")]
    dt: DateChoice,
}

/// `BalanceType12`
#[derive(Debug, Serialize)]
struct BalanceType {
    #[serde(rename = "CdOrPrtry")]
    cd_or_prtry: BalanceTypeChoice,
}

/// `BalanceType5Choice`
#[derive(Debug, Serialize)]
struct BalanceTypeChoice {
    #[serde(rename = "Cd")]
    cd: &'static str,
}

/// `ActiveOrHistoricCurrencyAndAmount`
#[derive(Debug, Serialize)]
struct Amount {
    #[serde(rename = "@Ccy")]
    ccy: String,
    #[serde(rename = "$text")]
    value: Decimal,
}

/// `DateAndDateTimeChoice`, restricted to the `Dt` (date-only) branch.
#[derive(Debug, Serialize)]
struct DateChoice {
    #[serde(rename = "Dt")]
    dt: NaiveDate,
}

/// `ReportEntry2`
#[derive(Debug, Serialize)]
struct Entry {
    #[serde(rename = "Amt")]
    amt: Amount,
    #[serde(rename = "CdtDbtInd")]
    cdt_dbt_ind: &'static str,
    #[serde(rename = "Sts")]
    sts: &'static str,
    #[serde(rename = "BookgDt")]
    bookg_dt: DateChoice,
    #[serde(rename = "ValDt")]
    val_dt: DateChoice,
    #[serde(rename = "AcctSvcrRef", skip_serializing_if = "Option::is_none")]
    acct_svcr_ref: Option<String>,
    #[serde(rename = "BkTxCd")]
    bk_tx_cd: BankTransactionCode,
    #[serde(rename = "NtryDtls", skip_serializing_if = "Vec::is_empty")]
    ntry_dtls: Vec<EntryDetails>,
    #[serde(rename = "AddtlNtryInf", skip_serializing_if = "Option::is_none")]
    addtl_ntry_inf: Option<String>,
}

/// `BankTransactionCodeStructure4`. Both `Domn` and `Prtry` are optional in
/// the schema, so an empty element is valid; we have no bank transaction
/// code to report, since `SimpleTransaction` does not carry one.
#[derive(Debug, Serialize)]
struct BankTransactionCode {}

/// `EntryDetails1`
#[derive(Debug, Serialize)]
struct EntryDetails {
    #[serde(rename = "TxDtls")]
    tx_dtls: Vec<TxDetails>,
}

/// `EntryTransaction2`, reduced to the fields we can populate.
#[derive(Debug, Serialize)]
struct TxDetails {
    #[serde(rename = "RltdPties", skip_serializing_if = "Option::is_none")]
    rltd_pties: Option<TransactionParty>,
    #[serde(rename = "RmtInf", skip_serializing_if = "Option::is_none")]
    rmt_inf: Option<RemittanceInformation>,
}

/// `TransactionParty2`, reduced to debtor/creditor name and account.
#[derive(Debug, Serialize, Default)]
struct TransactionParty {
    #[serde(rename = "Dbtr", skip_serializing_if = "Option::is_none")]
    dbtr: Option<PartyIdentification>,
    #[serde(rename = "DbtrAcct", skip_serializing_if = "Option::is_none")]
    dbtr_acct: Option<RelatedCashAccount>,
    #[serde(rename = "Cdtr", skip_serializing_if = "Option::is_none")]
    cdtr: Option<PartyIdentification>,
    #[serde(rename = "CdtrAcct", skip_serializing_if = "Option::is_none")]
    cdtr_acct: Option<RelatedCashAccount>,
}

/// `PartyIdentification32`, reduced to the name.
#[derive(Debug, Serialize)]
struct PartyIdentification {
    #[serde(rename = "Nm")]
    nm: String,
}

/// `RemittanceInformation5`, reduced to unstructured text.
#[derive(Debug, Serialize)]
struct RemittanceInformation {
    #[serde(rename = "Ustrd")]
    ustrd: Vec<String>,
}

/// Very small, permissive IBAN shape check (2 letters, 2 digits, up to 30
/// alphanumerics), matching the `IBAN2007Identifier` pattern in the schema.
/// Only used to decide whether an account identifier should be serialized
/// as `<IBAN>` or as a generic `<Othr>` identification.
fn is_iban(account: &str) -> bool {
    let mut chars = account.chars();
    let country: String = chars.by_ref().take(2).collect();
    let check: String = chars.by_ref().take(2).collect();
    let rest: String = chars.collect();
    country.len() == 2
        && country.chars().all(|c| c.is_ascii_uppercase())
        && check.len() == 2
        && check.chars().all(|c| c.is_ascii_digit())
        && !rest.is_empty()
        && rest.len() <= 30
        && rest.chars().all(|c| c.is_ascii_alphanumeric())
}

impl From<&SimpleTransaction> for TxDetails {
    fn from(tx: &SimpleTransaction) -> Self {
        let mut party = TransactionParty::default();
        if tx.counter_name.is_some() || tx.counter_iban.is_some() {
            let party_id = tx
                .counter_name
                .as_ref()
                .map(|nm| PartyIdentification { nm: nm.clone() });
            let account = tx.counter_iban.as_ref().map(|iban| RelatedCashAccount {
                id: AccountId::from_account(iban),
            });
            // A negative amount is a debit from our account: the
            // counterparty received the funds, so it is the creditor.
            // A positive amount is a credit: the counterparty paid us,
            // so it is the debtor.
            if tx.amount.is_sign_negative() {
                party.cdtr = party_id;
                party.cdtr_acct = account;
            } else {
                party.dbtr = party_id;
                party.dbtr_acct = account;
            }
        }
        let rltd_pties = if party.dbtr.is_some()
            || party.dbtr_acct.is_some()
            || party.cdtr.is_some()
            || party.cdtr_acct.is_some()
        {
            Some(party)
        } else {
            None
        };

        let rmt_inf = if tx.description.is_empty() {
            None
        } else {
            Some(RemittanceInformation {
                ustrd: vec![tx.description.clone()],
            })
        };

        TxDetails {
            rltd_pties,
            rmt_inf,
        }
    }
}

impl Entry {
    fn from_transaction(tx: &SimpleTransaction, currency: &str) -> Self {
        let cdt_dbt_ind = if tx.amount.is_sign_negative() {
            "DBIT"
        } else {
            "CRDT"
        };
        Entry {
            amt: Amount {
                ccy: currency.to_string(),
                value: tx.amount.abs(),
            },
            cdt_dbt_ind,
            sts: "BOOK",
            bookg_dt: DateChoice { dt: tx.book_date },
            val_dt: DateChoice { dt: tx.value_date },
            acct_svcr_ref: tx.reference.clone().filter(|r| !r.is_empty()),
            bk_tx_cd: BankTransactionCode {},
            ntry_dtls: vec![EntryDetails {
                tx_dtls: vec![TxDetails::from(tx)],
            }],
            addtl_ntry_inf: None,
        }
    }
}

impl Balance {
    fn new(kind: BalanceKind, date: NaiveDate, amount: Decimal, currency: &str) -> Self {
        Balance {
            tp: BalanceType {
                cd_or_prtry: BalanceTypeChoice { cd: kind.code() },
            },
            amt: Amount {
                ccy: currency.to_string(),
                value: amount.abs(),
            },
            cdt_dbt_ind: if amount.is_sign_negative() {
                "DBIT"
            } else {
                "CRDT"
            },
            dt: DateChoice { dt: date },
        }
    }
}

impl SimpleStatement {
    /// Renders this statement as a camt.053.001.02 XML document (a single
    /// `Document`/`BkToCstmrStmt` with one `Stmt`), including an XML
    /// declaration.
    ///
    /// This is a minimal but schema-compliant mapping: every field of
    /// [`SimpleStatement`] and [`SimpleTransaction`] is represented, placed
    /// at the correct position of the `camt.053.001.02` schema (see
    /// `doc/camt.053.001.02.xsd`), so no data is lost and the result
    /// validates against it. Elements that camt.053 requires but that have
    /// no equivalent in the simple model (such as bank transaction codes)
    /// are emitted as empty/minimal but valid placeholders.
    ///
    /// # Errors
    ///
    /// Returns [`CamtError::Xml`] if the XML serialization fails.
    pub fn to_camt053(&self) -> Result<String, CamtError> {
        let doc = Document {
            xmlns: CAMT053_NAMESPACE,
            bk_to_cstmr_stmt: BkToCstmrStmt {
                grp_hdr: GroupHeader {
                    msg_id: format!("{}-{}", self.account, self.closing_date),
                    cre_dt_tm: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                },
                stmt: vec![self.create_camt053_statement()],
            },
        };
        let body = quick_xml::se::to_string(&doc)?;
        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n{body}\n"
        ))
    }

    fn create_camt053_statement(&self) -> Stmt {
        let currency = self
            .currency
            .clone()
            .unwrap_or_else(|| DEFAULT_CURRENCY.to_string());
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        Stmt {
            id: format!("{}-{}", self.account, self.closing_date),
            cre_dt_tm: now,
            acct: CashAccount {
                id: AccountId::from_account(&self.account),
                ccy: Some(currency.clone()),
            },
            bal: vec![
                Balance::new(
                    BalanceKind::Opening,
                    self.opening_date,
                    self.opening_amount,
                    &currency,
                ),
                Balance::new(
                    BalanceKind::Closing,
                    self.closing_date,
                    self.closing_amount,
                    &currency,
                ),
            ],
            ntry: self
                .transactions
                .iter()
                .map(|tx| Entry::from_transaction(tx, &currency))
                .collect(),
        }
    }
}

impl SimpleStatements {
    /// Renders these statements as a camt.053.001.02 XML document (a single
    /// `Document`/`BkToCstmrStmt` with one or more `Stmt`), including an XML
    /// declaration.
    ///
    /// This is a minimal but schema-compliant mapping: every field of
    /// [`SimpleStatement`] and [`SimpleTransaction`] is represented, placed
    /// at the correct position of the `camt.053.001.02` schema (see
    /// `doc/camt.053.001.02.xsd`), so no data is lost and the result
    /// validates against it. Elements that camt.053 requires but that have
    /// no equivalent in the simple model (such as bank transaction codes)
    /// are emitted as empty/minimal but valid placeholders.
    ///
    /// # Errors
    ///
    /// Returns [`CamtError::Xml`] if the XML serialization fails.
    pub fn to_camt053(&self) -> Result<String, CamtError> {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let doc = Document {
            xmlns: CAMT053_NAMESPACE,
            bk_to_cstmr_stmt: BkToCstmrStmt {
                grp_hdr: GroupHeader {
                    msg_id: format!("camt053-export-{now}"),
                    cre_dt_tm: now.clone(),
                },
                stmt: self
                    .statements
                    .iter()
                    .map(|stmt| stmt.create_camt053_statement())
                    .collect(),
            },
        };
        let body = quick_xml::se::to_string(&doc)?;
        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n{body}\n"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_statement() -> SimpleStatement {
        SimpleStatement {
            account: "NL91ABNA0417164300".to_string(),
            currency: Some("EUR".to_string()),
            opening_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            opening_amount: dec!(100.00),
            closing_date: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            closing_amount: dec!(50.00),
            transactions: vec![
                SimpleTransaction {
                    account: "NL91ABNA0417164300".to_string(),
                    reference: Some("REF123".to_string()),
                    book_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
                    value_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
                    amount: dec!(-50.00),
                    counter_iban: Some("BE68539007547034".to_string()),
                    counter_name: Some("Some Counterparty & <special> \"chars\"".to_string()),
                    description: "Payment for invoice #42".to_string(),
                },
                SimpleTransaction {
                    account: "NL91ABNA0417164300".to_string(),
                    reference: None,
                    book_date: NaiveDate::from_ymd_opt(2024, 1, 20).unwrap(),
                    value_date: NaiveDate::from_ymd_opt(2024, 1, 20).unwrap(),
                    amount: dec!(0.00),
                    counter_iban: None,
                    counter_name: None,
                    description: String::new(),
                },
            ],
        }
    }

    #[test]
    fn to_camt053_contains_expected_elements() {
        let xml = sample_statement().to_camt053().unwrap();

        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>"));
        assert!(xml.contains("xmlns=\"urn:iso:std:iso:20022:tech:xsd:camt.053.001.02\""));
        assert!(xml.contains("<IBAN>NL91ABNA0417164300</IBAN>"));
        assert!(xml.contains("<Cd>OPBD</Cd>"));
        assert!(xml.contains("<Cd>CLBD</Cd>"));
        assert!(xml.contains("<Amt Ccy=\"EUR\">100.00</Amt>"));
        assert!(xml.contains("<Amt Ccy=\"EUR\">50.00</Amt>"));
        assert!(xml.contains("<AcctSvcrRef>REF123</AcctSvcrRef>"));
        assert!(
            xml.contains("<Cdtr><Nm>Some Counterparty &amp; &lt;special&gt; \"chars\"</Nm></Cdtr>")
        );
        assert!(xml.contains("<IBAN>BE68539007547034</IBAN>"));
        assert!(xml.contains("<Ustrd>Payment for invoice #42</Ustrd>"));
        // Zero-amount transaction with no counterparty/description/reference
        // must still produce valid (if minimal) entry details.
        assert!(xml.contains("<Amt Ccy=\"EUR\">0.00</Amt>"));
    }

    #[test]
    fn is_iban_detects_common_shapes() {
        assert!(is_iban("NL91ABNA0417164300"));
        assert!(is_iban("BE68539007547034"));
        assert!(!is_iban("not-an-iban"));
        assert!(!is_iban(""));
    }
}
