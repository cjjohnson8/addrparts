// Two-letter USPS state and territory codes. Used to check the state field
// without pulling in a real address database.
const STATE_CODES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA",
    "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
    "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT",
    "VA", "WA", "WV", "WI", "WY", "DC", "PR", "GU", "VI", "AS", "MP",
];

// Standard USPS street suffix abbreviations (Publication 28, Appendix C1),
// one preferred abbreviation per suffix type. Only used in --strict mode,
// since plenty of real-world input spells out "Street" or "Avenue" in full.
const STREET_SUFFIXES: &[&str] = &[
    "ALY", "ANX", "ARC", "AVE", "BYU", "BCH", "BND", "BLF", "BLFS", "BTM", "BLVD", "BR", "BRG",
    "BRK", "BRKS", "BG", "BGS", "BYP", "CP", "CYN", "CPE", "CSWY", "CTR", "CTRS", "CIR", "CIRS",
    "CLF", "CLFS", "CLB", "CMN", "CMNS", "COR", "CORS", "CRSE", "CT", "CTS", "CV", "CVS", "CRK",
    "CRES", "CRST", "XING", "XRD", "XRDS", "CURV", "DL", "DM", "DV", "DR", "DRS", "DRV", "EST",
    "ESTS", "EXPY", "EXT", "EXTS", "FALL", "FLS", "FRY", "FLD", "FLDS", "FLT", "FLTS", "FRD",
    "FRDS", "FRST", "FRG", "FRGS", "FRK", "FRKS", "FT", "FWY", "GDN", "GDNS", "GTWY", "GLN",
    "GLNS", "GRN", "GRNS", "GRV", "GRVS", "HBR", "HBRS", "HVN", "HTS", "HWY", "HL", "HLS",
    "HOLW", "INLT", "IS", "ISS", "ISLE", "JCT", "JCTS", "KY", "KYS", "KNL", "KNLS", "LK", "LKS",
    "LAND", "LNDG", "LN", "LGT", "LGTS", "LF", "LCK", "LCKS", "LDG", "LOOP", "MALL", "MNR",
    "MNRS", "MDW", "MDWS", "MEWS", "ML", "MLS", "MSN", "MTWY", "MT", "MTN", "MTNS", "NCK",
    "ORCH", "OVAL", "OPAS", "PARK", "PKWY", "PASS", "PSGE", "PATH", "PIKE", "PNE", "PNES", "PL",
    "PLN", "PLNS", "PLZ", "PT", "PTS", "PRT", "PRTS", "PR", "RADL", "RAMP", "RNCH", "RPD",
    "RPDS", "RST", "RDG", "RDGS", "RIV", "RD", "RDS", "RTE", "ROW", "RUE", "RUN", "SHL", "SHLS",
    "SHR", "SHRS", "SKWY", "SPG", "SPGS", "SPUR", "SQ", "SQS", "STA", "STRA", "STRM", "ST",
    "STS", "SMT", "TER", "TRWY", "TRCE", "TRAK", "TRFY", "TRL", "TRLR", "TUNL", "TPKE", "UPAS",
    "UN", "UNS", "VLY", "VLYS", "VIA", "VW", "VWS", "VLG", "VLGS", "VL", "VIS", "WALK", "WALKS",
    "WALL", "WAY", "WAYS", "WL", "WLS",
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
pub fn parse(input: &str, strict: bool) -> ParseOutcome {
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
    let primary_street = parts[0];
    let street = parts[..parts.len() - 2].join(", ");

    if strict {
        match primary_street.split_whitespace().last() {
            Some(word) if is_standard_suffix(word) => {}
            Some(word) => errors.push(format!(
                "'{word}' is not a standard USPS street suffix abbreviation"
            )),
            None => errors.push("street has no suffix to check in --strict mode".to_string()),
        }
    }

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

fn is_standard_suffix(word: &str) -> bool {
    let normalized = word.trim_end_matches('.').to_uppercase();
    STREET_SUFFIXES.contains(&normalized.as_str())
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
        let out = parse("123 Main St, Springfield, IL 62704", false);
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
        let out = parse("1600 Amphitheatre Pkwy, Mountain View, CA 94043-1351", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.zip4.as_deref(), Some("1351"));
    }

    #[test]
    fn folds_extra_segments_into_street() {
        let out = parse("500 Elm St, Apt 4B, Austin, TX 73301", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.street, "500 Elm St, Apt 4B");
    }

    #[test]
    fn rejects_bad_state() {
        let out = parse("1 First Ave, Nowhere, ZZ 00000", false);
        assert!(!out.is_valid());
    }

    #[test]
    fn rejects_short_input() {
        let out = parse("just a string", false);
        assert!(!out.is_valid());
        assert!(out.address.is_none());
    }

    #[test]
    fn strict_accepts_standard_suffix() {
        let out = parse("1600 Amphitheatre Pkwy, Mountain View, CA 94043", true);
        assert!(out.is_valid());
    }

    #[test]
    fn strict_accepts_suffix_with_trailing_period() {
        let out = parse("123 Main St., Springfield, IL 62704", true);
        assert!(out.is_valid());
    }

    #[test]
    fn strict_rejects_spelled_out_suffix() {
        let out = parse("123 Main Street, Springfield, IL 62704", true);
        assert!(!out.is_valid());
    }

    #[test]
    fn non_strict_ignores_spelled_out_suffix() {
        let out = parse("123 Main Street, Springfield, IL 62704", false);
        assert!(out.is_valid());
    }

    #[test]
    fn strict_checks_primary_street_not_folded_segment() {
        let out = parse("500 Elm St, Apt 4B, Austin, TX 73301", true);
        assert!(out.is_valid());
    }
}
