// Two-letter USPS state and territory codes. Used to check the state field
// without pulling in a real address database.
const STATE_CODES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA",
    "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
    "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT",
    "VA", "WA", "WV", "WI", "WY", "DC", "PR", "GU", "VI", "AS", "MP",
];

#[derive(Debug, Clone)]
pub struct ParsedAddress {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip5: String,
    pub zip4: Option<String>,
}

#[derive(Debug)]
pub struct ParseOutcome {
    pub input: String,
    pub address: Option<ParsedAddress>,
    pub errors: Vec<String>,
}

impl ParseOutcome {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty() && self.address.is_some()
    }
}

// Accepts the common single-line form:
//   STREET, CITY, STATE ZIP[-ZIP4]
// Anything before the last two comma segments is folded into the street
// field, since apartment/suite lines are sometimes comma-separated too.
pub fn parse(input: &str) -> ParseOutcome {
    let trimmed = input.trim();
    let mut errors = Vec::new();

    let parts: Vec<&str> = trimmed.split(',').map(|p| p.trim()).collect();
    if parts.len() < 3 || parts.iter().any(|p| p.is_empty()) {
        errors.push(
            "expected at least three non-empty comma-separated segments: street, city, state zip"
                .to_string(),
        );
        return ParseOutcome {
            input: trimmed.to_string(),
            address: None,
            errors,
        };
    }

    let last = parts[parts.len() - 1];
    let city = parts[parts.len() - 2].to_string();
    let street = parts[..parts.len() - 2].join(", ");

    let tail_tokens: Vec<&str> = last.split_whitespace().collect();
    if tail_tokens.len() < 2 {
        errors.push(format!(
            "could not split '{last}' into a state and a ZIP code"
        ));
        return ParseOutcome {
            input: trimmed.to_string(),
            address: None,
            errors,
        };
    }

    let zip_token = tail_tokens[tail_tokens.len() - 1];
    let state_raw = tail_tokens[..tail_tokens.len() - 1].join(" ").to_uppercase();

    if !STATE_CODES.contains(&state_raw.as_str()) {
        errors.push(format!("'{state_raw}' is not a recognized state or territory code"));
    }

    let (zip5, zip4) = match split_zip(zip_token) {
        Ok(pair) => pair,
        Err(e) => {
            errors.push(e);
            (String::new(), None)
        }
    };

    let address = ParsedAddress {
        street,
        city,
        state: state_raw,
        zip5,
        zip4,
    };

    ParseOutcome {
        input: trimmed.to_string(),
        address: Some(address),
        errors,
    }
}

fn split_zip(token: &str) -> Result<(String, Option<String>), String> {
    let (zip5, zip4) = match token.split_once('-') {
        Some((a, b)) => (a, Some(b)),
        None => (token, None),
    };

    if zip5.len() != 5 || !zip5.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("'{zip5}' is not a valid 5-digit ZIP code"));
    }

    if let Some(z4) = zip4 {
        if z4.len() != 4 || !z4.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("'{z4}' is not a valid 4-digit ZIP+4 suffix"));
        }
        Ok((zip5.to_string(), Some(z4.to_string())))
    } else {
        Ok((zip5.to_string(), None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_address() {
        let out = parse("123 Main St, Springfield, IL 62704");
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.street, "123 Main St");
        assert_eq!(addr.city, "Springfield");
        assert_eq!(addr.state, "IL");
        assert_eq!(addr.zip5, "62704");
        assert_eq!(addr.zip4, None);
    }

    #[test]
    fn parses_zip_plus_four() {
        let out = parse("1600 Amphitheatre Pkwy, Mountain View, CA 94043-1351");
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.zip4.as_deref(), Some("1351"));
    }

    #[test]
    fn folds_extra_segments_into_street() {
        let out = parse("500 Elm St, Apt 4B, Austin, TX 73301");
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.street, "500 Elm St, Apt 4B");
    }

    #[test]
    fn rejects_bad_state() {
        let out = parse("1 First Ave, Nowhere, ZZ 00000");
        assert!(!out.is_valid());
    }

    #[test]
    fn rejects_short_input() {
        let out = parse("just a string");
        assert!(!out.is_valid());
        assert!(out.address.is_none());
    }
}
