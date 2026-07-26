//! A Rust library for parsing **CAMT.053** (ISO 20022 Bank-to-Customer
//! Statement) XML files and converting them to **MT940** format.
//!
//! CAMT.053 is a verbose, deeply nested XML schema used by many European
//! banks to deliver account statements. This crate parses that schema and
//! reduces it to a small, ergonomic model ([`SimpleStatement`] /
//! [`SimpleTransaction`]) suitable for use in applications, plus a
//! convenience method to render the result as MT940 text for legacy
//! banking software.
//!
//! # Example
//!
//! ```no_run
//! use camt053::SimpleStatement;
//!
//! let statements = SimpleStatement::load("statement.xml")?;
//! for statement in &statements {
//!     println!("Account: {}", statement.account);
//!     for transaction in &statement.transactions {
//!         println!("{transaction}");
//!     }
//! }
//! # Ok::<(), camt053::CamtError>(())
//! ```
//!
//! [`SimpleStatement::load`] also accepts a `.zip` archive containing one or
//! more camt.053 XML files (as commonly delivered by banks for multi-day
//! exports); the contained statements are merged per account.
//!
//! # Feature overview
//!
//! - [`SimpleStatement::load`] — parse a `.xml` file (or `.zip` archive) into
//!   a list of [`SimpleStatement`]s, one per account.
//! - [`SimpleStatement::to_mt940`] — render a statement as an MT940 message.
//! - [`CamtError`] — the single error type returned by all fallible
//!   operations in this crate.

#![warn(missing_docs)]

mod camt053;
mod error;
mod simple;

pub use camt053::schema::{self, Document};
pub use error::CamtError;
pub use simple::{SimpleStatement, SimpleTransaction};
