//! Rust representation of the ISO 20022 `camt.053.001.02` (Bank-to-Customer
//! Statement) message, as defined by `camt.053.001.02.xsd` in `/doc`
//! and constrained by the Dutch Banking Association (DPA) Implementation
//! Guidelines for this message (used by ABN/SNS/ING and other NL banks).
//!
//! Struct/enum and field names use full, human-readable words rather than
//! the (often cryptic) ISO 20022 XML tag abbreviations. Each field carries
//! an explicit `#[serde(rename = "...")]` mapping to the original XSD/XML
//! tag name so the mapping to the schema stays traceable. Fields that are
//! `minOccurs="0"` in the XSD are `Option<T>`, repeatable elements are
//! `Vec<T>` (with `#[serde(default)]` when `minOccurs="0"`).
//!
//! A number of elements that only apply to securities/investment-fund
//! statements (e.g. `Tax`, `CorpActn`, `SfkpgAcct`, `RtrInf`, `RltdPric`,
//! `RltdQties`, `FinInstrmId`) are out of scope for a retail cash account
//! statement and are not modelled here.

use crate::error::CamtError;
use chrono::NaiveDate;
use serde::Deserialize;

mod date_format {
    const FORMAT: &str = "%Y-%m-%d";

    // Used for the `Dt` alternative of a `DateAndDateTimeChoice`, which is
    // optional (the `DtTm` alternative may be used instead).
    pub mod option {
        use super::FORMAT;
        use chrono::NaiveDate;
        use serde::{self, Deserialize, Deserializer};

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            NaiveDate::parse_from_str(&s, FORMAT)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
    }
}

/// Root `<Document>` element.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Document {
    // Namespaced attribute on the root element; other `xmlns`/`xmlns:xsi`
    // attributes on <Document> are ignored (no `deny_unknown_fields` here).
    /// Schema location attribute on the root `<Document>` element, if present.
    #[serde(rename = "@schemaLocation")]
    pub schema_location: Option<String>,
    /// Bank-to-customer statement payload carried by this document.
    #[serde(rename = "BkToCstmrStmt")]
    pub bank_to_customer_statement: BankToCustomerStatement,
}

/// `BkToCstmrStmt` - Bank to Customer Statement.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BankToCustomerStatement {
    /// Group header for the statement message.
    #[serde(rename = "GrpHdr")]
    pub group_header: GroupHeader,
    /// Statements included in the message.
    #[serde(rename = "Stmt")]
    pub statements: Vec<Statement>,
}

/// `GrpHdr` - Group Header.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GroupHeader {
    /// Message identifier for this group header.
    #[serde(rename = "MsgId")]
    pub message_identification: String,
    // #[serde(with = "date_format")]
    /// Creation date and time of the message.
    #[serde(rename = "CreDtTm")]
    pub creation_date_time: String, //NaiveDateTime,
    /// Recipient of the message, if present.
    #[serde(rename = "MsgRcpt")]
    pub message_recipient: Option<PartyIdentification>,
    /// Pagination details for the message, if present.
    #[serde(rename = "MsgPgntn")]
    pub message_pagination: Option<Pagination>,
    /// Additional information for the group header, if present.
    #[serde(rename = "AddtlInf")]
    pub additional_information: Option<String>,
}

/// `Pgntn` - Pagination.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Pagination {
    /// Page number within the paginated message.
    #[serde(rename = "PgNb")]
    pub page_number: String,
    /// Whether this is the last page of the message.
    #[serde(rename = "LastPgInd")]
    pub last_page_indicator: bool,
}

/// `Stmt` - Statement.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    /// Identifier of the statement.
    #[serde(rename = "Id")]
    pub identification: String,
    /// Electronic sequence number of the statement, if present.
    #[serde(rename = "ElctrncSeqNb")]
    pub electronic_sequence_number: Option<String>,
    /// Legal sequence number of the statement, if present.
    #[serde(rename = "LglSeqNb")]
    pub legal_sequence_number: Option<String>,
    /// Creation date and time of the statement.
    #[serde(rename = "CreDtTm")]
    pub creation_date_time: String, //NaiveDateTime,
    /// Date and time range covered by the statement, if present.
    #[serde(rename = "FrToDt")]
    pub from_to_date: Option<FromToDate>,
    /// Whether the statement is an original, copy, or duplicate, if present.
    #[serde(rename = "CpyDplctInd")]
    pub copy_duplicate_indicator: Option<String>,
    /// Source from which the statement was reported, if present.
    #[serde(rename = "RptgSrc")]
    pub reporting_source: Option<ReportingSourceChoice>,
    /// Account to which the statement applies.
    #[serde(rename = "Acct")]
    pub account: Account,
    /// Related account referenced by the statement, if present.
    #[serde(rename = "RltdAcct")]
    pub related_account: Option<Account>,
    /// Interest information reported for the account.
    #[serde(rename = "Intrst", default)]
    pub interest: Vec<AccountInterest>,
    /// Balances reported on the statement.
    #[serde(rename = "Bal")]
    pub balance: Vec<Balance>,
    /// Summary totals for the statement entries, if present.
    #[serde(rename = "TxsSummry")]
    pub transactions_summary: Option<TransactionsSummary>,
    /// Entries reported on the statement.
    #[serde(rename = "Ntry", default)]
    pub entry: Vec<Entry>,
    /// Additional statement information, if present.
    #[serde(rename = "AddtlStmtInf")]
    pub additional_statement_information: Option<String>,
}

/// `FrToDt` - From/To Date.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FromToDate {
    /// Start date and time of the covered period.
    #[serde(rename = "FrDtTm")]
    pub from_date_time: String, //NaiveDateTime,
    /// End date and time of the covered period.
    #[serde(rename = "ToDtTm")]
    pub to_date_time: String, //NaiveDateTime,
}

/// `RptgSrc` - Reporting Source (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReportingSourceChoice {
    /// Code value for this reporting source.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this reporting source, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `Intrst` - Account Interest.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccountInterest {
    /// Interest type for this account interest, if present.
    #[serde(rename = "Tp")]
    pub interest_type: Option<InterestTypeChoice>,
    /// Rates reported for this account interest.
    #[serde(rename = "Rate", default)]
    pub rate: Vec<Rate>,
    /// Start and end dates for this account interest, if present.
    #[serde(rename = "FrToDt")]
    pub from_to_date: Option<FromToDate>,
    /// Reason for this account interest, if present.
    #[serde(rename = "Rsn")]
    pub reason: Option<String>,
}

/// `Tp` - Interest Type (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterestTypeChoice {
    /// Code value for this interest type.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this interest type, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `Rate` - Interest Rate.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rate {
    /// Rate type for this interest rate.
    #[serde(rename = "Tp")]
    pub rate_type: RateTypeChoice,
}

/// `Tp` - Rate Type (choice of `Pctg`/`Othr`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RateTypeChoice {
    /// Percentage rate for this rate type, if present.
    #[serde(rename = "Pctg")]
    pub percentage: Option<f64>,
    /// Non-percentage rate type value, if present.
    #[serde(rename = "Othr")]
    pub other: Option<String>,
}

/// `Acct` - Cash Account.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// Identifier for this cash account.
    #[serde(rename = "Id")]
    pub identification: AccountIdentification,
    /// Type of the account, if present.
    #[serde(rename = "Tp")]
    pub account_type: Option<AccountType>,
    /// Currency of this cash account, if present.
    #[serde(rename = "Ccy")]
    pub currency: Option<String>,
    /// Name for this cash account, if present.
    #[serde(rename = "Nm")]
    pub name: Option<String>,
    /// Owner of the account, if present.
    #[serde(rename = "Ownr")]
    pub owner: Option<PartyIdentification>,
    /// Servicing financial institution for the account, if present.
    #[serde(rename = "Svcr")]
    pub servicer: Option<Agent>,
}

/// `Id` - Cash Account Identification (choice of `IBAN`/`Othr`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccountIdentification {
    /// IBAN of the account, if the identification scheme is IBAN.
    #[serde(rename = "IBAN", default)]
    pub iban: Option<String>,
    /// Other account identification, if the account is not identified by IBAN.
    #[serde(rename = "Othr")]
    pub other: Option<GenericAccountIdentification>,
}

/// `Othr` - Generic Account Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GenericAccountIdentification {
    /// Identifier for this generic account identification.
    #[serde(rename = "Id")]
    pub identification: String,
    /// Scheme name for this generic account identification, if present.
    #[serde(rename = "SchmeNm")]
    pub scheme_name: Option<AccountSchemeNameChoice>,
    /// Issuer of this generic account identification, if present.
    #[serde(rename = "Issr")]
    pub issuer: Option<String>,
}

/// `SchmeNm` - Account Scheme Name (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccountSchemeNameChoice {
    /// Code value for this account scheme name.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this account scheme name, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `Tp` - Cash Account Type (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccountType {
    /// Code value for this cash account type.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this cash account type, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `Svcr`/`DbtrAgt`/`CdtrAgt`/... - Financial Institution / Branch reference.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Agent {
    /// Financial institution identification for the agent.
    #[serde(rename = "FinInstnId")]
    pub financial_institution_identification: FinancialInstitutionIdentification,
    /// Branch identification for the agent, if present.
    #[serde(rename = "BrnchId")]
    pub branch_identification: Option<BranchData>,
}

/// `BrnchId` - Branch Data.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BranchData {
    /// Identifier for this branch data.
    #[serde(rename = "Id")]
    pub identification: Option<String>,
    /// Name for this branch data, if present.
    #[serde(rename = "Nm")]
    pub name: Option<String>,
    /// Postal address for this branch data, if present.
    #[serde(rename = "PstlAdr")]
    pub postal_address: Option<PostalAddress>,
}

/// `FinInstnId` - Financial Institution Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FinancialInstitutionIdentification {
    /// BIC of the financial institution, if present.
    #[serde(rename = "BIC", default)]
    pub bic: Option<String>,
    /// Clearing system member identification for this financial institution identification, if present.
    #[serde(rename = "ClrSysMmbId")]
    pub clearing_system_member_identification: Option<ClearingSystemMemberIdentification>,
    /// Name for this financial institution identification, if present.
    #[serde(rename = "Nm")]
    pub name: Option<String>,
    /// Postal address for this financial institution identification, if present.
    #[serde(rename = "PstlAdr")]
    pub postal_address: Option<PostalAddress>,
    /// Other identification of the financial institution, if present.
    #[serde(rename = "Othr")]
    pub other: Option<GenericFinancialIdentification>,
}

/// `ClrSysMmbId` - Clearing System Member Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClearingSystemMemberIdentification {
    /// Clearing system identification for this clearing system member identification, if present.
    #[serde(rename = "ClrSysId")]
    pub clearing_system_identification: Option<ClearingSystemIdentificationChoice>,
    /// Member identification for this clearing system member identification.
    #[serde(rename = "MmbId")]
    pub member_identification: String,
}

/// `ClrSysId` - Clearing System Identification (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClearingSystemIdentificationChoice {
    /// Code value for this clearing system identification.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this clearing system identification, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `Othr` - Generic Financial Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GenericFinancialIdentification {
    /// Identifier for this generic financial identification.
    #[serde(rename = "Id")]
    pub identification: String,
    /// Scheme name for this generic financial identification, if present.
    #[serde(rename = "SchmeNm")]
    pub scheme_name: Option<FinancialIdentificationSchemeNameChoice>,
    /// Issuer of this generic financial identification, if present.
    #[serde(rename = "Issr")]
    pub issuer: Option<String>,
}

/// `SchmeNm` - Financial Identification Scheme Name (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FinancialIdentificationSchemeNameChoice {
    /// Code value for this financial identification scheme name.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this financial identification scheme name, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `PstlAdr` (`PostalAddress6`) - shared by parties, agents and branches.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PostalAddress {
    /// Address type for this shared by parties, agents and branches, if present.
    #[serde(rename = "AdrTp")]
    pub address_type: Option<String>,
    /// Department for this shared by parties, agents and branches, if present.
    #[serde(rename = "Dept")]
    pub department: Option<String>,
    /// Sub department for this shared by parties, agents and branches, if present.
    #[serde(rename = "SubDept")]
    pub sub_department: Option<String>,
    /// Street name for this shared by parties, agents and branches, if present.
    #[serde(rename = "StrtNm")]
    pub street_name: Option<String>,
    /// Building number for this shared by parties, agents and branches, if present.
    #[serde(rename = "BldgNb")]
    pub building_number: Option<String>,
    /// Post code for this shared by parties, agents and branches, if present.
    #[serde(rename = "PstCd")]
    pub post_code: Option<String>,
    /// Town name for this shared by parties, agents and branches, if present.
    #[serde(rename = "TwnNm")]
    pub town_name: Option<String>,
    /// Country sub division for this shared by parties, agents and branches, if present.
    #[serde(rename = "CtrySubDvsn")]
    pub country_sub_division: Option<String>,
    /// Country for the postal address, if present.
    #[serde(rename = "Ctry")]
    pub country: Option<String>,
    /// Unstructured address lines.
    #[serde(rename = "AdrLine", default)]
    pub address_line: Vec<String>,
}

/// `Ownr`/`MsgRcpt`/`Dbtr`/`Cdtr`/... - Party Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartyIdentification {
    /// Name for this party identification, if present.
    #[serde(rename = "Nm")]
    pub name: Option<String>,
    /// Postal address for this party identification, if present.
    #[serde(rename = "PstlAdr")]
    pub postal_address: Option<PostalAddress>,
    /// Identification of the party, if present.
    #[serde(rename = "Id")]
    pub identification: Option<PartyChoice>,
    /// Country of residence of the party, if present.
    #[serde(rename = "CtryOfRes")]
    pub country_of_residence: Option<String>,
    /// Contact details for the party, if present.
    #[serde(rename = "CtctDtls")]
    pub contact_details: Option<ContactDetails>,
}

/// `Id` - Party Identification (choice of `OrgId`/`PrvtId`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartyChoice {
    /// Organisation identification for the party, if present.
    #[serde(rename = "OrgId")]
    pub organisation_identification: Option<OrganisationIdentification>,
    /// Private identification for the party, if present.
    #[serde(rename = "PrvtId")]
    pub private_identification: Option<PersonIdentification>,
}

/// `OrgId` - Organisation Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OrganisationIdentification {
    /// BIC or BEI identifying the organisation, if present.
    #[serde(rename = "BICOrBEI", default)]
    pub bic_or_bei: Option<String>,
    /// Additional alternate identifications for this organisation identification.
    #[serde(rename = "Othr", default)]
    pub other: Vec<GenericOrganisationIdentification>,
}

/// `Othr` - Generic Organisation Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GenericOrganisationIdentification {
    /// Identifier for this generic organisation identification.
    #[serde(rename = "Id")]
    pub identification: String,
    /// Scheme name for this generic organisation identification, if present.
    #[serde(rename = "SchmeNm")]
    pub scheme_name: Option<OrganisationIdentificationSchemeNameChoice>,
    /// Issuer of this generic organisation identification, if present.
    #[serde(rename = "Issr")]
    pub issuer: Option<String>,
}

/// `SchmeNm` - Organisation Identification Scheme Name (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OrganisationIdentificationSchemeNameChoice {
    /// Code value for this organisation identification scheme name.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this organisation identification scheme name, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `PrvtId` - Person Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PersonIdentification {
    /// Date and place of birth of the person, if present.
    #[serde(rename = "DtAndPlcOfBirth")]
    pub date_and_place_of_birth: Option<DateAndPlaceOfBirth>,
    /// Additional alternate identifications for this person identification.
    #[serde(rename = "Othr", default)]
    pub other: Vec<GenericPersonIdentification>,
}

/// `Othr` - Generic Person Identification.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GenericPersonIdentification {
    /// Identifier for this generic person identification.
    #[serde(rename = "Id")]
    pub identification: String,
    /// Scheme name for this generic person identification, if present.
    #[serde(rename = "SchmeNm")]
    pub scheme_name: Option<PersonIdentificationSchemeNameChoice>,
    /// Issuer of this generic person identification, if present.
    #[serde(rename = "Issr")]
    pub issuer: Option<String>,
}

/// `SchmeNm` - Person Identification Scheme Name (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PersonIdentificationSchemeNameChoice {
    /// Code value for this person identification scheme name.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this person identification scheme name, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `DtAndPlcOfBirth` - Date and Place of Birth.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DateAndPlaceOfBirth {
    /// Birth date for this date and place of birth.
    #[serde(rename = "BirthDt")]
    pub birth_date: String,
    /// Province of birth for this date and place of birth, if present.
    #[serde(rename = "PrvcOfBirth")]
    pub province_of_birth: Option<String>,
    /// City of birth for this date and place of birth.
    #[serde(rename = "CityOfBirth")]
    pub city_of_birth: String,
    /// Country of birth for this date and place of birth.
    #[serde(rename = "CtryOfBirth")]
    pub country_of_birth: String,
}

/// `CtctDtls` - Contact Details.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContactDetails {
    /// Name prefix for this contact details, if present.
    #[serde(rename = "NmPrfx")]
    pub name_prefix: Option<String>,
    /// Name for this contact details, if present.
    #[serde(rename = "Nm")]
    pub name: Option<String>,
    /// Phone number for this contact details, if present.
    #[serde(rename = "PhneNb")]
    pub phone_number: Option<String>,
    /// Mobile number for this contact details, if present.
    #[serde(rename = "MobNb")]
    pub mobile_number: Option<String>,
    /// Fax number for this contact details, if present.
    #[serde(rename = "FaxNb")]
    pub fax_number: Option<String>,
    /// Email address for this contact details, if present.
    #[serde(rename = "EmailAdr")]
    pub email_address: Option<String>,
    /// Other contact detail, if present.
    #[serde(rename = "Othr")]
    pub other: Option<String>,
}

/// `TxsSummry` - Transactions Summary.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionsSummary {
    /// Overall totals for all entries, if present.
    #[serde(rename = "TtlNtries")]
    pub total_entries: Option<TotalEntries>,
    /// Totals for credit entries, if present.
    #[serde(rename = "TtlCdtNtries")]
    pub total_credit_entries: Option<NumberAndSumOfTransactions>,
    /// Totals for debit entries, if present.
    #[serde(rename = "TtlDbtNtries")]
    pub total_debit_entries: Option<NumberAndSumOfTransactions>,
    /// Totals grouped by bank transaction code.
    #[serde(rename = "TtlNtriesPerBkTxCd", default)]
    pub total_entries_per_bank_transaction_code: Vec<TotalsPerBankTransactionCode>,
}

/// `TtlNtries` - Total Entries.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TotalEntries {
    /// Number of entries included in this total entries, if present.
    #[serde(rename = "NbOfNtries")]
    pub number_of_entries: Option<u32>,
    /// Summed amount for this total entries, if present.
    #[serde(rename = "Sum")]
    pub sum: Option<f64>,
    /// Net amount for this total entries, if present.
    #[serde(rename = "TtlNetNtryAmt")]
    pub total_net_entry_amount: Option<f64>,
    /// Whether this total entries is credit or debit, if present.
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: Option<CreditDebitIndicator>,
}

/// `TtlCdtNtries`/`TtlDbtNtries` - Number and Sum of Transactions.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NumberAndSumOfTransactions {
    /// Number of entries included in this number and sum of transactions, if present.
    #[serde(rename = "NbOfNtries")]
    pub number_of_entries: Option<u32>,
    /// Summed amount for this number and sum of transactions, if present.
    #[serde(rename = "Sum")]
    pub sum: Option<f64>,
}

/// `TtlNtriesPerBkTxCd` - Totals per Bank Transaction Code.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TotalsPerBankTransactionCode {
    /// Number of entries included in this totals per bank transaction code, if present.
    #[serde(rename = "NbOfNtries")]
    pub number_of_entries: Option<u32>,
    /// Summed amount for this totals per bank transaction code, if present.
    #[serde(rename = "Sum")]
    pub sum: Option<f64>,
    /// Net amount for this totals per bank transaction code, if present.
    #[serde(rename = "TtlNetNtryAmt")]
    pub total_net_entry_amount: Option<f64>,
    /// Whether this totals per bank transaction code is credit or debit, if present.
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: Option<CreditDebitIndicator>,
    /// Whether the amount is forecast, if present.
    #[serde(rename = "FcstInd")]
    pub forecast_indicator: Option<bool>,
    /// Bank transaction code for this totals per bank transaction code.
    #[serde(rename = "BkTxCd")]
    pub bank_transaction_code: BankTransactionCodeStructure,
    /// Availability breakdowns for this bank transaction code.
    #[serde(rename = "Avlbty", default)]
    pub availability: Vec<CashBalanceAvailability>,
}

/// `Bal` - Cash Balance.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Balance {
    /// Type of balance being reported.
    #[serde(rename = "Tp")]
    pub balance_type: BalanceType,
    /// Credit line associated with the balance, if present.
    #[serde(rename = "CdtLine")]
    pub credit_line: Option<CreditLine>,
    /// Amount of the balance.
    #[serde(rename = "Amt")]
    pub amount: f64,
    /// Whether the balance is credit or debit.
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: CreditDebitIndicator,
    /// Date associated with the balance.
    #[serde(rename = "Dt")]
    pub date: DateChoice,
}

/// `CdtLine` - Credit Line.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreditLine {
    /// Whether the credit line is included in the balance.
    #[serde(rename = "Incl")]
    pub included: bool,
    /// Amount of the credit line, if present.
    #[serde(rename = "Amt")]
    pub amount: Option<f64>,
}

/// `Tp` - Balance Type.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BalanceType {
    /// Standard or proprietary value for this balance type.
    #[serde(rename = "CdOrPrtry")]
    pub code_or_proprietary: CodeOrProprietary,
    /// Sub-type for this balance type, if present.
    #[serde(rename = "SubTp")]
    pub sub_type: Option<BalanceSubTypeChoice>,
}

/// `SubTp` - Balance Sub Type (choice of `Cd`/`Prtry`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BalanceSubTypeChoice {
    /// Code value for this balance sub type.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this balance sub type, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `CdOrPrtry` - Balance Type Code or Proprietary.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CodeOrProprietary {
    /// Standard ISO balance type code, if present.
    #[serde(rename = "Cd")]
    pub code: Option<BalanceTypeCode>,
    /// Proprietary balance type value, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `BalanceType12Code` - Balance type code values.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub enum BalanceTypeCode {
    /// Expected closing booked balance.
    XPCD, // Expected Closing Booked Balance
    /// Opening available balance.
    OPAV, // Opening Available Balance
    /// Interim available balance.
    ITAV, // Interim Available Balance
    /// Closing available balance.
    CLAV, // Closing Available Balance - Closing balance of amount of money that is at the disposal of the account owner on the date specified
    /// Forward available balance.
    FWAV, // Forward Available Balance - Forward available balance of amount of money that is at the disposal of the account owner on the date specified
    /// Closing booked balance.
    CLBD, // Closing Booked Balance - Balance of the account at the end of the pre-agreed account reporting period.
    /// Interim booked balance.
    ITBD, // Interim Booked Balance
    /// Opening booked balance.
    OPBD, // Opening Booked Balance
    /// Previous closing booked balance.
    PRCD, // Closing book balance of the previous day - Represents the closing book balance of the previous day.
    /// Informational balance.
    INFO, // Information Balance
}

/// `DateAndDateTimeChoice` - `Dt` (date only) or `DtTm` (date and time).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DateChoice {
    /// Calendar date value, if the XML used `<Dt>`.
    #[serde(rename = "Dt", default, with = "date_format::option")]
    pub date: Option<NaiveDate>,
    /// Date-time value, if the XML used `<DtTm>`.
    #[serde(rename = "DtTm")]
    pub date_time: Option<String>,
}

impl DateChoice {
    /// Returns the calendar date, regardless of whether the source used
    /// `<Dt>` (date only) or `<DtTm>` (date and time).
    pub fn date(&self) -> Result<NaiveDate, CamtError> {
        if let Some(date) = self.date {
            return Ok(date);
        }
        let date_time = self.date_time.as_ref().ok_or(CamtError::MissingDate)?;
        let prefix = date_time
            .get(..10)
            .ok_or_else(|| CamtError::InvalidDateTime(date_time.clone()))?;
        NaiveDate::parse_from_str(prefix, "%Y-%m-%d")
            .map_err(|_| CamtError::InvalidDateTime(date_time.clone()))
    }
}

/// `CreditDebitCode` - Credit/Debit indicator.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub enum CreditDebitIndicator {
    /// Debit entry or balance.
    DBIT,
    /// Credit entry or balance.
    CRDT,
}

/// `EntryStatus2Code` - Entry status.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Deserialize, PartialEq)]
pub enum EntryStatus {
    /// Booked entry.
    BOOK,
    /// Pending entry.
    PDNG,
    /// Informational entry.
    INFO,
}

/// `Ntry` - Entry.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Reference assigned to the entry, if present.
    #[serde(rename = "NtryRef")]
    pub entry_reference: Option<String>,
    /// Amount of the entry.
    #[serde(rename = "Amt")]
    pub amount: f64,
    /// Whether the entry is credit or debit.
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: CreditDebitIndicator,
    /// Whether the entry is a reversal, if present.
    #[serde(rename = "RvslInd")]
    pub reversal_indicator: Option<bool>,
    /// Booking status of the entry.
    #[serde(rename = "Sts")]
    pub status: EntryStatus,
    /// Booking date of the entry.
    #[serde(rename = "BookgDt")]
    pub booking_date: DateChoice,
    /// Value date of the entry.
    #[serde(rename = "ValDt")]
    pub value_date: DateChoice,
    /// Reference assigned by the account servicer, if present.
    #[serde(rename = "AcctSvcrRef")]
    pub account_servicer_reference: Option<String>,
    /// Bank transaction code for the entry, if present.
    #[serde(rename = "BkTxCd")]
    pub bank_transaction_code: Option<BankTransactionCodeStructure>,
    /// Whether commission was waived for the entry, if present.
    #[serde(rename = "ComssnWvrInd")]
    pub commission_waiver_indicator: Option<bool>,
    /// Technical input channel for the entry, if present.
    #[serde(rename = "TechInptChanl")]
    pub technical_input_channel: Option<TechnicalInputChannelChoice>,
    /// Further details for the entry.
    #[serde(rename = "NtryDtls", default)]
    pub details: Vec<EntryDetails>,
    /// Additional information about the entry, if present.
    #[serde(rename = "AddtlNtryInf")]
    pub additional_information: Option<String>,
}

/// `TechInptChanl` - Technical Input Channel (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TechnicalInputChannelChoice {
    /// Code value for this technical input channel.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this technical input channel, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `BkTxCd` - Bank Transaction Code Structure.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BankTransactionCodeStructure {
    /// Domain part of the bank transaction code, if present.
    #[serde(rename = "Domn")]
    pub domain: Option<BankTransactionCodeStructureDomain>,
    /// Proprietary value for this bank transaction code structure, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<ProprietaryBankTransactionCodeStructure>,
}

/// `Domn` - Bank Transaction Code Domain.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BankTransactionCodeStructureDomain {
    /// Code value for this bank transaction code domain.
    #[serde(rename = "Cd")]
    pub code: String,
    /// Family part of the bank transaction code.
    #[serde(rename = "Fmly")]
    pub family: BankTransactionCodeStructureFamily,
}

/// `Fmly` - Bank Transaction Code Family.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BankTransactionCodeStructureFamily {
    /// Code value for this bank transaction code family.
    #[serde(rename = "Cd")]
    pub code: String,
    /// Sub-family code within the bank transaction code family.
    #[serde(rename = "SubFmlyCd")]
    pub sub_family_code: String,
}

/// `Prtry` - Proprietary Bank Transaction Code Structure.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProprietaryBankTransactionCodeStructure {
    /// Code value for this proprietary bank transaction code structure.
    #[serde(rename = "Cd")]
    pub code: String,
    /// Issuer of this proprietary bank transaction code structure, if present.
    #[serde(rename = "Issr")]
    pub issuer: Option<String>,
}

/// `Avlbty` - Cash Balance Availability.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CashBalanceAvailability {
    /// Date for this cash balance availability.
    #[serde(rename = "Dt")]
    pub date: CashBalanceAvailabilityDate,
    /// Amount for this cash balance availability.
    #[serde(rename = "Amt")]
    pub amount: f64,
    /// Whether this cash balance availability is credit or debit.
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: CreditDebitIndicator,
}

/// `Dt` - Cash Balance Availability Date (choice of `NbOfDays`/`ActlDt`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CashBalanceAvailabilityDate {
    /// Number of days for this cash balance availability date, if present.
    #[serde(rename = "NbOfDays")]
    pub number_of_days: Option<String>,
    /// Actual date for this cash balance availability date, if present.
    #[serde(rename = "ActlDt")]
    pub actual_date: Option<String>,
}

/// `NtryDtls` - Entry Details.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EntryDetails {
    /// Batch information for this entry detail, if present.
    #[serde(rename = "Btch")]
    pub batch: Option<BatchInformation>,
    /// Transaction details contained in this entry detail.
    #[serde(rename = "TxDtls", default)]
    pub transaction_details: Vec<TransactionDetails>,
}

/// `Btch` - Batch Information.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BatchInformation {
    /// Message identification for this batch information, if present.
    #[serde(rename = "MsgId")]
    pub message_identification: Option<String>,
    /// Payment information identifier, if present.
    #[serde(rename = "PmtInfId")]
    pub payment_information_identification: Option<String>,
    /// Number of transactions in the batch, if present.
    #[serde(rename = "NbOfTxs")]
    pub number_of_transactions: Option<String>,
    /// Total amount of the batch, if present.
    #[serde(rename = "TtlAmt")]
    pub total_amount: Option<f64>,
    /// Whether this batch information is credit or debit, if present.
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: Option<CreditDebitIndicator>,
}

/// `Amt` with a `Ccy` attribute (`ActiveOrHistoricCurrencyAndAmount`).
#[derive(Debug, Deserialize, PartialEq)]
pub struct CurrencyAndAmount {
    // The '@' prefix tells quick-xml to look for an attribute named 'Ccy'
    /// Currency code carried by the amount.
    #[serde(rename = "@Ccy")]
    pub currency: String,

    // The '$value' identifier captures the inner text content of the tag
    /// Numeric value of the amount.
    #[serde(rename = "$value")]
    pub value: f64,
}

/// `Amount` - Instructed Amount or Transaction Amount.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Amount {
    /// Currency and value of the amount.
    #[serde(rename = "Amt")]
    pub amount: CurrencyAndAmount,
}

/// `AmtDtls` - Amount Details.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AmountDetails {
    /// Instructed amount for this amount details, if present.
    #[serde(rename = "InstdAmt")]
    pub instructed_amount: Option<Amount>,
    /// Transaction amount for this amount details, if present.
    #[serde(rename = "TxAmt")]
    pub transaction_amount: Option<Amount>,
}

/// `TxDtls` - Transaction Details.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionDetails {
    /// References associated with the transaction, if present.
    #[serde(rename = "Refs")]
    pub references: Option<TransactionReferences>,
    /// Amount breakdown for the transaction, if present.
    #[serde(rename = "AmtDtls")]
    pub amount_details: Option<AmountDetails>,
    /// Bank transaction code for the transaction, if present.
    #[serde(rename = "BkTxCd")]
    pub bank_transaction_code: Option<BankTransactionCodeStructure>,
    /// Parties related to the transaction, if present.
    #[serde(rename = "RltdPties")]
    pub related_parties: Option<RelatedParties>,
    /// Agents related to the transaction, if present.
    #[serde(rename = "RltdAgts")]
    pub related_agents: Option<TransactionAgents>,
    /// Purpose of the transaction, if present.
    #[serde(rename = "Purp")]
    pub purpose: Option<PurposeChoice>,
    /// Locations where related remittance information can be found.
    #[serde(rename = "RltdRmtInf", default)]
    pub related_remittance_information: Vec<RemittanceLocation>,
    /// Remittance information for the transaction, if present.
    #[serde(rename = "RmtInf")]
    pub remittance_information: Option<RemittanceInformation>,
    /// Dates related to the transaction, if present.
    #[serde(rename = "RltdDts")]
    pub related_dates: Option<TransactionDates>,
    /// Additional transaction information, if present.
    #[serde(rename = "AddtlTxInf")]
    pub additional_transaction_information: Option<String>,
}

/// `Refs` - Transaction References.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionReferences {
    /// Message identification for this transaction references, if present.
    #[serde(rename = "MsgId")]
    pub message_identification: Option<String>,
    /// Account servicer reference for this transaction references, if present.
    #[serde(rename = "AcctSvcrRef")]
    pub account_servicer_reference: Option<String>,
    /// Payment information identifier, if present.
    #[serde(rename = "PmtInfId")]
    pub payment_information_identification: Option<String>,
    /// Instruction identification for this transaction references, if present.
    #[serde(rename = "InstrId")]
    pub instruction_identification: Option<String>,
    /// End to end identification for this transaction references, if present.
    #[serde(rename = "EndToEndId")]
    pub end_to_end_identification: Option<String>,
    /// Transaction identification for this transaction references, if present.
    #[serde(rename = "TxId")]
    pub transaction_identification: Option<String>,
    /// Mandate identification for this transaction references, if present.
    #[serde(rename = "MndtId")]
    pub mandate_identification: Option<String>,
    /// Cheque number for this transaction references, if present.
    #[serde(rename = "ChqNb")]
    pub cheque_number: Option<String>,
    /// Clearing system reference for this transaction references, if present.
    #[serde(rename = "ClrSysRef")]
    pub clearing_system_reference: Option<String>,
    /// Proprietary value for this transaction references, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<ProprietaryReference>,
}

/// `Prtry` - Proprietary Reference.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProprietaryReference {
    /// Type of reference used in this proprietary reference.
    #[serde(rename = "Tp")]
    pub reference_type: String,
    /// Reference value for this proprietary reference.
    #[serde(rename = "Ref")]
    pub reference: String,
}

/// `RltdAgts` - Transaction Agents.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionAgents {
    /// Debtor for the transaction, if present.
    #[serde(rename = "DbtrAgt")]
    pub debtor_agent: Option<Agent>,
    /// Creditor for the transaction, if present.
    #[serde(rename = "CdtrAgt")]
    pub creditor_agent: Option<Agent>,
    /// Intermediary agent 1 for the transaction, if present.
    #[serde(rename = "IntrmyAgt1")]
    pub intermediary_agent1: Option<Agent>,
    /// Intermediary agent 2 for the transaction, if present.
    #[serde(rename = "IntrmyAgt2")]
    pub intermediary_agent2: Option<Agent>,
    /// Intermediary agent 3 for the transaction, if present.
    #[serde(rename = "IntrmyAgt3")]
    pub intermediary_agent3: Option<Agent>,
    /// Receiving for the transaction, if present.
    #[serde(rename = "RcvgAgt")]
    pub receiving_agent: Option<Agent>,
    /// Delivering for the transaction, if present.
    #[serde(rename = "DlvrgAgt")]
    pub delivering_agent: Option<Agent>,
    /// Issuing for the transaction, if present.
    #[serde(rename = "IssgAgt")]
    pub issuing_agent: Option<Agent>,
    /// Settlement place agent for the transaction, if present.
    #[serde(rename = "SttlmPlc")]
    pub settlement_place: Option<Agent>,
    /// Additional proprietary agent roles.
    #[serde(rename = "Prtry", default)]
    pub proprietary: Vec<ProprietaryAgent>,
}

/// `Prtry` - Proprietary Agent.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProprietaryAgent {
    /// Type of this proprietary agent role.
    #[serde(rename = "Tp")]
    pub agent_type: String,
    /// Agent associated with this proprietary agent role.
    #[serde(rename = "Agt")]
    pub agent: Agent,
}

/// `Purp` - Purpose (choice of `Cd`/`Prtry`).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PurposeChoice {
    /// Code value for this purpose.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this purpose, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `RltdRmtInf` - Remittance Location.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RemittanceLocation {
    /// Remittance identification for this remittance location, if present.
    #[serde(rename = "RmtId")]
    pub remittance_identification: Option<String>,
    /// Remittance location method for this remittance location, if present.
    #[serde(rename = "RmtLctnMtd")]
    pub remittance_location_method: Option<String>,
    /// Remittance location electronic address for this remittance location, if present.
    #[serde(rename = "RmtLctnElctrncAdr")]
    pub remittance_location_electronic_address: Option<String>,
    /// Remittance location postal address for this remittance location, if present.
    #[serde(rename = "RmtLctnPstlAdr")]
    pub remittance_location_postal_address: Option<NameAndAddress>,
}

/// `RmtLctnPstlAdr` - Name and Address.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NameAndAddress {
    /// Name part of the remittance location address.
    #[serde(rename = "Nm")]
    pub name: String,
    /// Postal address part of the remittance location address.
    #[serde(rename = "Adr")]
    pub address: PostalAddress,
}

/// `RltdDts` - Transaction Dates.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionDates {
    /// Acceptance date time for this transaction dates, if present.
    #[serde(rename = "AccptncDtTm")]
    pub acceptance_date_time: Option<String>,
    /// Trade activity contractual settlement date for this transaction dates, if present.
    #[serde(rename = "TradActvtyCtrctlSttlmDt")]
    pub trade_activity_contractual_settlement_date: Option<String>,
    /// Trade date for this transaction dates, if present.
    #[serde(rename = "TradDt")]
    pub trade_date: Option<String>,
    /// Interbank settlement date for this transaction dates, if present.
    #[serde(rename = "IntrBkSttlmDt")]
    pub interbank_settlement_date: Option<String>,
    /// Start date for this transaction dates, if present.
    #[serde(rename = "StartDt")]
    pub start_date: Option<String>,
    /// End date for this transaction dates, if present.
    #[serde(rename = "EndDt")]
    pub end_date: Option<String>,
    /// Transaction date time for this transaction dates, if present.
    #[serde(rename = "TxDtTm")]
    pub transaction_date_time: Option<String>,
    /// Proprietary values for this transaction dates.
    #[serde(rename = "Prtry", default)]
    pub proprietary: Vec<ProprietaryDate>,
}

/// `Prtry` - Proprietary Date.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProprietaryDate {
    /// Date type for this proprietary date.
    #[serde(rename = "Tp")]
    pub date_type: String,
    /// Date for this proprietary date.
    #[serde(rename = "Dt")]
    pub date: DateOrDateTime,
}

/// `Dt` - Date or Date Time.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DateOrDateTime {
    /// Date for this date or date time.
    #[serde(rename = "Dt")]
    pub date: Option<String>,
    /// Date-time for this date or date time, if present.
    #[serde(rename = "DtTm")]
    pub date_time: Option<String>,
}

/// `RltdPties` - Related Parties.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RelatedParties {
    /// Initiating party for this related parties, if present.
    #[serde(rename = "InitgPty")]
    pub initiating_party: Option<PartyIdentification>,
    /// Debtor for this related parties, if present.
    #[serde(rename = "Dbtr")]
    pub debtor: Option<PartyIdentification>,
    /// Debtor account for this related parties, if present.
    #[serde(rename = "DbtrAcct")]
    pub debtor_account: Option<Account>,
    /// Ultimate debtor for this related parties, if present.
    #[serde(rename = "UltmtDbtr")]
    pub ultimate_debtor: Option<PartyIdentification>,
    /// Creditor for this related parties, if present.
    #[serde(rename = "Cdtr")]
    pub creditor: Option<PartyIdentification>,
    /// Creditor account for this related parties, if present.
    #[serde(rename = "CdtrAcct")]
    pub creditor_account: Option<Account>,
    /// Ultimate creditor for this related parties, if present.
    #[serde(rename = "UltmtCdtr")]
    pub ultimate_creditor: Option<PartyIdentification>,
    /// Trading party for this related parties, if present.
    #[serde(rename = "TradgPty")]
    pub trading_party: Option<PartyIdentification>,
    /// Proprietary values for this related parties.
    #[serde(rename = "Prtry", default)]
    pub proprietary: Vec<ProprietaryParty>,
}

/// `Prtry` - Proprietary Party.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProprietaryParty {
    /// Type of this proprietary party role.
    #[serde(rename = "Tp")]
    pub party_type: String,
    /// Party associated with this proprietary party role.
    #[serde(rename = "Pty")]
    pub party: PartyIdentification,
}

/// `RmtInf` - Remittance Information.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RemittanceInformation {
    /// Unstructured remittance information lines.
    #[serde(rename = "Ustrd", default)]
    pub unstructured: Vec<String>,
    /// Structured remittance information blocks.
    #[serde(rename = "Strd", default)]
    pub structured: Vec<StructuredRemittanceInformation>,
}

/// `Strd` - Structured Remittance Information.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StructuredRemittanceInformation {
    /// Referenced document details.
    #[serde(rename = "RfrdDocInf", default)]
    pub referred_document_information: Vec<ReferredDocumentInformation>,
    /// Amounts related to the referenced documents, if present.
    #[serde(rename = "RfrdDocAmt")]
    pub referred_document_amount: Option<RemittanceAmount>,
    /// Creditor reference information, if present.
    #[serde(rename = "CdtrRefInf")]
    pub creditor_reference_information: Option<CreditorReferenceInformation>,
    /// Party issuing the invoice, if present.
    #[serde(rename = "Invcr")]
    pub invoicer: Option<PartyIdentification>,
    /// Party receiving the invoice, if present.
    #[serde(rename = "Invcee")]
    pub invoicee: Option<PartyIdentification>,
    /// Additional remittance information lines.
    #[serde(rename = "AddtlRmtInf", default)]
    pub additional_remittance_information: Vec<String>,
}

/// `RfrdDocInf` - Referred Document Information.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferredDocumentInformation {
    /// Type of the referenced document, if present.
    #[serde(rename = "Tp")]
    pub doc_type: Option<ReferredDocumentType>,
    /// Number for this referred document information, if present.
    #[serde(rename = "Nb")]
    pub number: Option<String>,
    /// Date related to this referred document information, if present.
    #[serde(rename = "RltdDt")]
    pub related_date: Option<String>,
}

/// `Tp` - Referred Document Type.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferredDocumentType {
    /// Standard or proprietary value for this referred document type.
    #[serde(rename = "CdOrPrtry")]
    pub code_or_proprietary: ReferredDocumentTypeChoice,
    /// Issuer of this referred document type, if present.
    #[serde(rename = "Issr")]
    pub issuer: Option<String>,
}

/// `CdOrPrtry` - Referred Document Type Code or Proprietary.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferredDocumentTypeChoice {
    /// Code value for this referred document type code or proprietary.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this referred document type code or proprietary, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}

/// `RfrdDocAmt` - Remittance Amount.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RemittanceAmount {
    /// Amount that remains due and payable, if present.
    #[serde(rename = "DuePyblAmt")]
    pub due_payable_amount: Option<f64>,
    /// Discount amount applied, if present.
    #[serde(rename = "DscntApldAmt")]
    pub discount_applied_amount: Option<f64>,
    /// Credit note amount, if present.
    #[serde(rename = "CdtNoteAmt")]
    pub credit_note_amount: Option<f64>,
    /// Tax amount, if present.
    #[serde(rename = "TaxAmt")]
    pub tax_amount: Option<f64>,
    /// Adjustment amounts and their reasons.
    #[serde(rename = "AdjstmntAmtAndRsn", default)]
    pub adjustment_amount_and_reason: Vec<DocumentAdjustment>,
    /// Amount actually remitted, if present.
    #[serde(rename = "RmtdAmt")]
    pub remitted_amount: Option<f64>,
}

/// `AdjstmntAmtAndRsn` - Document Adjustment.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DocumentAdjustment {
    /// Amount for this document adjustment.
    #[serde(rename = "Amt")]
    pub amount: f64,
    /// Whether this document adjustment is credit or debit, if present.
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: Option<CreditDebitIndicator>,
    /// Reason for this document adjustment, if present.
    #[serde(rename = "Rsn")]
    pub reason: Option<String>,
    /// Additional information for this document adjustment, if present.
    #[serde(rename = "AddtlInf")]
    pub additional_information: Option<String>,
}

/// `CdtrRefInf` - Creditor Reference Information.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreditorReferenceInformation {
    /// Type of reference used in this creditor reference information, if present.
    #[serde(rename = "Tp")]
    pub reference_type: Option<CreditorReferenceType>,
    /// Reference value for this creditor reference information, if present.
    #[serde(rename = "Ref")]
    pub reference: Option<String>,
}

/// `Tp` - Creditor Reference Type.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreditorReferenceType {
    /// Standard or proprietary value for this creditor reference type.
    #[serde(rename = "CdOrPrtry")]
    pub code_or_proprietary: CreditorReferenceTypeChoice,
    /// Issuer of this creditor reference type, if present.
    #[serde(rename = "Issr")]
    pub issuer: Option<String>,
}

/// `CdOrPrtry` - Creditor Reference Type Code or Proprietary.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreditorReferenceTypeChoice {
    /// Code value for this creditor reference type code or proprietary.
    #[serde(rename = "Cd")]
    pub code: Option<String>,
    /// Proprietary value for this creditor reference type code or proprietary, if present.
    #[serde(rename = "Prtry")]
    pub proprietary: Option<String>,
}
