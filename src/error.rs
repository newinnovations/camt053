//! Centralized error type for this crate.
//!
//! All fallible operations (file I/O, XML parsing, zip handling and the
//! camt.053 -> [`crate::simple::SimpleStatement`] reduction) return
//! [`CamtError`] instead of panicking, so callers can
//! decide how to report failures.

use chrono::NaiveDate;
use rust_decimal::Decimal;

/// The single error type returned by all fallible operations in this crate.
#[derive(Debug, thiserror::Error)]
pub enum CamtError {
    /// Reading the input file (or an entry within a `.zip` archive) failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Opening or reading a `.zip` archive failed.
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// The input could not be parsed as valid camt.053 XML.
    #[error("Failed to parse camt.053 XML: {0}")]
    Xml(#[from] quick_xml::DeError),

    /// A statement's `<Acct><Id>` has neither an `<IBAN>` nor an `<Othr>`
    /// identification.
    #[error("Statement has no account identification (neither IBAN nor Othr)")]
    MissingIdentification,

    /// A statement is missing a required balance (e.g. opening or closing).
    #[error("Statement for account {account} has no {kind} balance")]
    MissingBalance {
        /// IBAN (or other identification) of the affected account.
        account: String,
        /// Kind of balance that is missing, e.g. `"opening"` or `"closing"`.
        kind: &'static str,
    },

    /// A date element has neither a `<Dt>` nor a `<DtTm>` child.
    #[error("Invalid date: neither Dt nor DtTm is present")]
    MissingDate,

    /// A `<DtTm>` value does not start with a parseable `YYYY-MM-DD` date
    /// prefix.
    #[error("Invalid DtTm date prefix: {0}")]
    InvalidDateTime(String),

    /// An entry has no `<NtryDtls><TxDtls>` block and no `<AddtlNtryInf>`
    /// fallback, so no description can be derived.
    #[error("Entry for account {account} has no transaction details to derive a description from")]
    MissingTransactionDetails {
        /// IBAN (or other identification) of the affected account.
        account: String,
    },

    /// An entry has more than one `<TxDtls>` block, which is not supported.
    #[error("Entry for account {account} has {count} transaction detail(s), expected exactly one")]
    UnexpectedTransactionDetailsCount {
        /// IBAN (or other identification) of the affected account.
        account: String,
        /// Number of `<TxDtls>` blocks found.
        count: usize,
    },

    /// Two statements for the same account, being merged from a `.zip`
    /// archive, overlap in time (the next one opens before the previous one
    /// closes).
    #[error(
        "Overlapping camt.053 statements for account {account}: one closes on {prev_closing} \
         but the next opens on {next_opening}"
    )]
    OverlappingStatements {
        /// IBAN (or other identification) of the affected account.
        account: String,
        /// Closing date of the earlier statement.
        prev_closing: NaiveDate,
        /// Opening date of the later statement.
        next_opening: NaiveDate,
    },

    /// Two consecutive statements for the same account, being merged from a
    /// `.zip` archive, have a closing balance that does not match the next
    /// statement's opening balance.
    #[error(
        "Balance gap in camt.053 statements for account {account}: closing balance {prev_closing_amount:.2} \
         on {prev_closing_date} does not match opening balance {next_opening_amount:.2} on {next_opening_date}"
    )]
    BalanceGap {
        /// IBAN (or other identification) of the affected account.
        account: String,
        /// Closing date of the earlier statement.
        prev_closing_date: NaiveDate,
        /// Closing balance of the earlier statement.
        prev_closing_amount: Decimal,
        /// Opening date of the later statement.
        next_opening_date: NaiveDate,
        /// Opening balance of the later statement.
        next_opening_amount: Decimal,
    },

    /// Two consecutive statements for the same account, being merged from a
    /// `.zip` archive, have different currencies.
    #[error(
        "Currency mismatch in camt.053 statements for account {account}: previous statement has currency {prev_currency:?} \
         but next statement has currency {next_currency:?}"
    )]
    CurrencyMismatch {
        /// IBAN (or other identification) of the affected account.
        account: String,
        /// Currency of the previous statement.
        prev_currency: Option<String>,
        /// Currency of the next statement.
        next_currency: Option<String>,
    },

    /// A statement's opening balance, closing balance and the sum of its
    /// transactions do not add up.
    #[error(
        "Inconsistent camt.053 statement for account {account}: opening balance {opening_amount:.2} on {opening_date} \
         does not match closing balance {closing_amount:.2} on {closing_date}, with a transaction sum of {transaction_sum:.2}"
    )]
    InconsistentStatement {
        /// IBAN (or other identification) of the affected account.
        account: String,
        /// Opening date of the statement.
        opening_date: NaiveDate,
        /// Opening balance of the statement.
        opening_amount: Decimal,
        /// Closing date of the statement.
        closing_date: NaiveDate,
        /// Closing balance of the statement.
        closing_amount: Decimal,
        /// Sum of all transaction amounts in the statement.
        transaction_sum: Decimal,
    },
}
