//! MT940 export, built on top of the bank agnostic [`crate::simple`]
//! structs rather than directly on the camt.053 schema types.

use crate::simple::{SimpleStatement, SimpleTransaction};
use chrono::NaiveDate;

impl SimpleStatement {
    /// Renders this statement as a single MT940 message.
    pub fn to_mt940(&self) -> String {
        let mut stmt_940 = String::default();

        stmt_940 += "{1:F01SNSBNL2AXXXX0000000000}{2:O940SNSBNL2AXXXXN}{3:}{4:\r\n";
        stmt_940 += ":20:0000000000\r\n";
        stmt_940 += &format!(":25:{}\r\n", self.account);
        stmt_940 += &format!(
            "{}\r\n",
            balance_tag_940("60F", self.opening_date, self.opening_amount)
        );

        let mut amount = self.opening_amount;
        for transaction in &self.transactions {
            amount += transaction.amount;
            stmt_940 += &format!("{}\r\n", transaction.entry_940_61());
            stmt_940 += &format!("{}\r\n", transaction.entry_940_86());
        }

        stmt_940 += &format!("{}\r\n", balance_tag_940("62F", self.closing_date, amount));
        stmt_940 += "-}{5:}\r\n";
        stmt_940
    }

    /// Suggested filename for the MT940 export of this statement, based on
    /// its opening and closing date.
    pub fn filename_940(&self) -> String {
        format!(
            "{}_{}_{}.940",
            self.account,
            self.opening_date.format("%Y-%m-%d"),
            self.closing_date.format("%Y-%m-%d"),
        )
    }
}

impl SimpleTransaction {
    fn entry_940_61(&self) -> String {
        let amount = if self.amount < 0.0 {
            format!("D{:.2}", -self.amount)
        } else {
            format!("C{:.2}", self.amount)
        };
        let amount = amount.replace('.', ",");

        format!(
            ":61:{}{}{}NDIV",
            self.value_date.format("%y%m%d"),
            self.book_date.format("%m%d"),
            amount
        )
    }

    fn entry_940_86(&self) -> String {
        format!(":86:{}", self.description)
    }
}

/// Formats a `60F`/`62F` style balance tag: `:<tag>:<C|D><yymmdd>EUR<amount>`.
fn balance_tag_940(tag: &str, date: NaiveDate, amount: f64) -> String {
    let date = date.format("%y%m%d");
    let res = if amount < 0.0 {
        format!(":{tag}:D{date}EUR{:.2}", -amount)
    } else {
        format!(":{tag}:C{date}EUR{:.2}", amount)
    };
    res.replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camt053::import_camt_from_reader;
    use crate::simple;

    fn test_statement() -> SimpleStatement {
        let doc = import_camt_from_reader(simple::TEST_XML.as_bytes()).expect("valid test fixture");
        simple::model::convert(&doc)
            .expect("valid statement")
            .remove(0)
    }

    #[test]
    fn balance_tag_940_formats_credit_and_debit() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 23).unwrap();
        assert_eq!(
            balance_tag_940("60F", date, 203.69),
            ":60F:C260123EUR203,69"
        );
        assert_eq!(
            balance_tag_940("60F", date, -203.69),
            ":60F:D260123EUR203,69"
        );
    }

    #[test]
    fn entry_940_61_formats_debit_and_credit_entries() {
        let statement = test_statement();
        assert_eq!(
            statement.transactions[0].entry_940_61(),
            ":61:2603050305D100,00NDIV"
        );
        assert_eq!(
            statement.transactions[1].entry_940_61(),
            ":61:2604100410C50,00NDIV"
        );
    }

    #[test]
    fn entry_940_86_uses_raw_description() {
        let statement = test_statement();
        assert_eq!(
            statement.transactions[0].entry_940_86(),
            ":86:Payment for invoice 123"
        );
    }

    #[test]
    fn to_mt940_produces_expected_header_and_balances() {
        let statement = test_statement();
        let mt940 = statement.to_mt940();
        assert!(mt940.starts_with("{1:F01SNSBNL2AXXXX0000000000}{2:O940SNSBNL2AXXXXN}{3:}{4:\r\n"));
        assert!(mt940.contains(":25:NL00SNSB0000000000\r\n"));
        assert!(mt940.contains(":60F:C260101EUR1000,00\r\n"));
        // Opening 1000.00 - 100.00 (debit) + 50.00 (credit) = 950.00
        assert!(mt940.contains(":62F:C260721EUR950,00\r\n"));
        assert!(mt940.ends_with("-}{5:}\r\n"));
    }

    #[test]
    fn filename_940_uses_opening_and_closing_dates() {
        let statement = test_statement();
        assert_eq!(
            statement.filename_940(),
            "NL00SNSB0000000000_2026-01-01_2026-07-21.940"
        );
    }
}
