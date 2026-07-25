use std::collections::HashMap;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct SepaTransaction {
    pub transaction_type: Option<String>,
    pub creditor_scheme_identifier: Option<String>,
    pub creditor_name: Option<String>,
    pub mandate_reference: Option<String>,
    pub remittance_info: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,
    pub reference: Option<String>,
}

impl SepaTransaction {
    /// Parses a raw SEPA string into a structured SepaTransaction struct
    pub fn parse(raw_desc: &str) -> Self {
        // List of official SEPA tags we want to look for
        let tags = [
            "/TRTP/", "/CSID/", "/NAME/", "/MARF/", "/REMI/", "/IBAN/", "/BIC/", "/EREF/",
        ];
        let mut extracted_data: HashMap<&str, String> = HashMap::new();

        // 1. Find the starting position of every tag present in the text
        let mut positions: Vec<(&str, usize)> = tags
            .iter()
            .filter_map(|&tag| raw_desc.find(tag).map(|idx| (tag, idx)))
            .collect();

        // 2. Sort positions by where they appear in the string
        positions.sort_by_key(|&(_, idx)| idx);

        // 3. Extract the text between each tag
        for i in 0..positions.len() {
            let (current_tag, current_idx) = positions[i];
            let start_value_idx = current_idx + current_tag.len();

            // The value ends either at the start of the next tag, or at the end of the string
            let end_value_idx = if i + 1 < positions.len() {
                positions[i + 1].1
            } else {
                raw_desc.len()
            };

            // Slice out the text, trim spaces, and save it
            let value = raw_desc[start_value_idx..end_value_idx].trim().to_string();
            extracted_data.insert(current_tag, value);
        }

        // 4. Map the HashMap values back into our structured object
        SepaTransaction {
            transaction_type: extracted_data.remove("/TRTP/"),
            creditor_scheme_identifier: extracted_data.remove("/CSID/"),
            creditor_name: extracted_data.remove("/NAME/"),
            mandate_reference: extracted_data.remove("/MARF/"),
            remittance_info: extracted_data.remove("/REMI/"),
            iban: extracted_data.remove("/IBAN/"),
            bic: extracted_data.remove("/BIC/"),
            reference: extracted_data.remove("/EREF/"),
        }
    }
}

// 12345678901234567890123456789012
// RENTE EN/OF KOSTEN
// CREDITRENTE               19,76C
// van 31.12.2025 tot 31.03.2026
// ZAKELIJK FLEXIBEL SPAREN
// Kijk voor de rentepercentages op
// www.abnamro.nl/allesoverrente
//
pub fn abnamro_clean_description(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + input.len() / 32);
    let mut last_was_space = false;

    for (count, c) in input.chars().enumerate() {
        // 1. Insert a space at every 32nd position
        if count > 0 && count % 32 == 0 && !last_was_space {
            result.push(' ');
            last_was_space = true;
        }

        // 2. Add the character, deduplicating consecutive spaces
        if c == ' ' {
            if !last_was_space {
                result.push(c);
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }

    result
}
