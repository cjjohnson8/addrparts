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
    // Populated when the street line is a PO box rather than a delivery
    // address ("PO Box 123", "P.O. Box 123", "Post Office Box 123", ...).
    // `street` still holds the full original text either way.
    pub po_box: Option<String>,
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

impl ParsedAddress {
    // Rejoins the parsed fields into the same single-line form the parser
    // accepts as input. Useful for normalizing whitespace and casing (the
    // state code is upper-cased during parsing) without hand-editing the
    // original string.
    pub fn to_single_line(&self) -> String {
        match &self.zip4 {
            Some(zip4) => format!(
                "{}, {}, {} {}-{}",
                self.street, self.city, self.state, self.zip5, zip4
            ),
            None => format!("{}, {}, {} {}", self.street, self.city, self.state, self.zip5),
        }
    }
}

// Accepts the common single-line form:
//   STREET, CITY, STATE ZIP[-ZIP4]
// Anything before the last two comma segments is folded into the street
// field, since apartment/suite lines are sometimes comma-separated too.
//
// If there aren't enough comma segments, falls back to whitespace-only
// parsing (see parse_whitespace_fallback) rather than giving up outright.
pub fn parse(input: &str, strict: bool) -> ParseOutcome {
    let trimmed = input.trim();
    let mut errors = Vec::new();

    let parts: Vec<&str> = trimmed.split(',').map(|p| p.trim()).collect();
    let (street, primary_street, city, tail) = if parts.len() < 3 || parts.iter().any(|p| p.is_empty()) {
        match parse_whitespace_fallback(trimmed) {
            Some((street, city, tail)) => (street.clone(), street, city, tail),
            None => {
                errors.push(
                    "expected at least three non-empty comma-separated segments (street, city, \
                     state zip), or a whitespace-only address ending in STATE ZIP with a \
                     recognizable street suffix"
                        .to_string(),
                );
                return ParseOutcome {
                    input: trimmed.to_string(),
                    address: None,
                    errors,
                };
            }
        }
    } else {
        let last = parts[parts.len() - 1].to_string();
        let city = parts[parts.len() - 2].to_string();
        let street = parts[..parts.len() - 2].join(", ");
        let primary_street = parts[0].to_string();
        (street, primary_street, city, last)
    };

    let po_box_match = detect_po_box(&primary_street);
    let po_box = match &po_box_match {
        Some(Some(number)) => Some(number.clone()),
        Some(None) => {
            errors.push("PO box line has no box number".to_string());
            None
        }
        None => None,
    };

    // A PO box has no street suffix to check, so --strict skips it there.
    if strict && po_box_match.is_none() {
        match primary_street.split_whitespace().last() {
            Some(word) if is_standard_suffix(word) => {}
            Some(word) => errors.push(format!(
                "'{word}' is not a standard USPS street suffix abbreviation"
            )),
            None => errors.push("street has no suffix to check in --strict mode".to_string()),
        }
    }

    let tail_tokens: Vec<&str> = tail.split_whitespace().collect();
    if tail_tokens.len() < 2 {
        errors.push(format!(
            "could not split '{tail}' into a state and a ZIP code"
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
        po_box,
    };

    ParseOutcome {
        input: trimmed.to_string(),
        address: Some(address),
        errors,
    }
}

// Splits a comma-less (or comma-starved) address into street/city/"state
// zip" by whitespace alone. The last two tokens are assumed to be STATE and
// ZIP. To find where the street ends and the city begins, it looks for the
// rightmost token among what's left that matches a standard USPS suffix
// abbreviation (St, Ave, Pkwy, ...) - the same list --strict checks against.
// Without a comma, that's the only structural signal available; addresses
// where the suffix is spelled out in full (e.g. "Street") can't be split
// this way and are left to fail with an error.
fn parse_whitespace_fallback(trimmed: &str) -> Option<(String, String, String)> {
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    // Minimum viable shape: one street word, a suffix, a city word, a
    // state, and a ZIP.
    if tokens.len() < 5 {
        return None;
    }

    let state_idx = tokens.len() - 2;
    let middle = &tokens[..state_idx];

    let mut suffix_idx = None;
    for (i, tok) in middle.iter().enumerate() {
        if i == middle.len() - 1 {
            // Leave at least one token for the city.
            break;
        }
        if is_standard_suffix(tok) {
            suffix_idx = Some(i);
        }
    }

    let suffix_idx = suffix_idx?;
    let street = middle[..=suffix_idx].join(" ");
    let city = middle[suffix_idx + 1..].join(" ");
    let tail = format!("{} {}", tokens[state_idx], tokens[tokens.len() - 1]);
    Some((street, city, tail))
}

// Splits on newlines and parses each non-blank line independently, for
// batch input piped in one address per line.
pub fn parse_lines(input: &str, strict: bool) -> Vec<ParseOutcome> {
    input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| parse(line, strict))
        .collect()
}

// Recognizes a PO box line and pulls out the box number.
//   Some(Some(number)) - matched a PO box prefix, number extracted
//   Some(None)         - matched a PO box prefix, but nothing followed it
//   None                - not a PO box line at all
//
// Periods are stripped before matching so "P.O. Box", "PO Box", and
// "P.O.Box" (no space before "Box") all normalize the same way. "P O Box"
// (each letter its own token) is also accepted since it shows up often
// enough in scraped data.
fn detect_po_box(street: &str) -> Option<Option<String>> {
    let words: Vec<&str> = street.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }

    let normalized: Vec<String> = words
        .iter()
        .map(|w| w.chars().filter(|c| *c != '.').collect::<String>().to_uppercase())
        .collect();

    let prefix_len = if normalized[0] == "POBOX" {
        Some(1)
    } else if starts_with_seq(&normalized, &["PO", "BOX"]) {
        Some(2)
    } else if starts_with_seq(&normalized, &["P", "O", "BOX"]) {
        Some(3)
    } else if starts_with_seq(&normalized, &["POST", "OFFICE", "BOX"]) {
        Some(3)
    } else {
        None
    }?;

    let number = words[prefix_len..].join(" ");
    if number.is_empty() {
        Some(None)
    } else {
        Some(Some(number))
    }
}

fn starts_with_seq(normalized: &[String], seq: &[&str]) -> bool {
    normalized.len() >= seq.len() && normalized.iter().zip(seq).all(|(a, b)| a == b)
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

    #[test]
    fn parse_lines_skips_blank_lines() {
        let input = "123 Main St, Springfield, IL 62704\n\n   \n1 First Ave, Nowhere, ZZ 00000\n";
        let outcomes = parse_lines(input, false);
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes[0].is_valid());
        assert!(!outcomes[1].is_valid());
    }

    #[test]
    fn parse_lines_trims_each_line() {
        let outcomes = parse_lines("  123 Main St, Springfield, IL 62704  \n", false);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].input, "123 Main St, Springfield, IL 62704");
    }

    #[test]
    fn parse_lines_empty_input_yields_no_outcomes() {
        assert!(parse_lines("\n\n", false).is_empty());
    }

    #[test]
    fn to_single_line_reassembles_without_zip4() {
        let out = parse("123 Main St, Springfield, IL 62704", false);
        let addr = out.address.unwrap();
        assert_eq!(addr.to_single_line(), "123 Main St, Springfield, IL 62704");
    }

    #[test]
    fn to_single_line_reassembles_with_zip4() {
        let out = parse("1600 Amphitheatre Pkwy, Mountain View, CA 94043-1351", false);
        let addr = out.address.unwrap();
        assert_eq!(
            addr.to_single_line(),
            "1600 Amphitheatre Pkwy, Mountain View, CA 94043-1351"
        );
    }

    #[test]
    fn whitespace_fallback_parses_address_without_commas() {
        let out = parse("123 Main St Springfield IL 62704", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.street, "123 Main St");
        assert_eq!(addr.city, "Springfield");
        assert_eq!(addr.state, "IL");
        assert_eq!(addr.zip5, "62704");
    }

    #[test]
    fn whitespace_fallback_handles_zip_plus_four() {
        let out = parse("1600 Amphitheatre Pkwy Mountain View CA 94043-1351", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.street, "1600 Amphitheatre Pkwy");
        assert_eq!(addr.city, "Mountain View");
        assert_eq!(addr.zip4.as_deref(), Some("1351"));
    }

    #[test]
    fn whitespace_fallback_picks_rightmost_suffix_match() {
        // "Park" is itself a standard suffix abbreviation, so the fallback
        // must not stop there when a later, better boundary exists.
        let out = parse("1 Main Park Ave Parkville OH 43000", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.street, "1 Main Park Ave");
        assert_eq!(addr.city, "Parkville");
    }

    #[test]
    fn whitespace_fallback_fails_without_recognizable_suffix() {
        let out = parse("PO Box 123 Springfield IL 62704", false);
        assert!(!out.is_valid());
        assert!(out.address.is_none());
    }

    #[test]
    fn whitespace_fallback_respects_strict_mode() {
        let out = parse("123 Main St Springfield IL 62704", true);
        assert!(out.is_valid());
    }

    #[test]
    fn detects_plain_po_box() {
        let out = parse("PO Box 456, Austin, TX 73301", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.po_box.as_deref(), Some("456"));
        assert_eq!(addr.street, "PO Box 456");
    }

    #[test]
    fn detects_po_box_with_periods() {
        let out = parse("P.O. Box 123, Austin, TX 73301", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.po_box.as_deref(), Some("123"));
    }

    #[test]
    fn detects_po_box_spelled_out() {
        let out = parse("Post Office Box 789, Austin, TX 73301", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.po_box.as_deref(), Some("789"));
    }

    #[test]
    fn detects_po_box_spaced_out() {
        let out = parse("P O Box 42, Austin, TX 73301", false);
        assert!(out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.po_box.as_deref(), Some("42"));
    }

    #[test]
    fn regular_street_has_no_po_box() {
        let out = parse("123 Main St, Springfield, IL 62704", false);
        let addr = out.address.unwrap();
        assert_eq!(addr.po_box, None);
    }

    #[test]
    fn po_box_missing_number_is_invalid() {
        let out = parse("PO Box, Austin, TX 73301", false);
        assert!(!out.is_valid());
        let addr = out.address.unwrap();
        assert_eq!(addr.po_box, None);
    }

    #[test]
    fn strict_mode_skips_suffix_check_for_po_box() {
        let out = parse("PO Box 123, Austin, TX 73301", true);
        assert!(out.is_valid());
    }

    #[test]
    fn to_single_line_uppercases_state_and_keeps_folded_segments() {
        let out = parse("500 Elm St, Apt 4B, Austin, tx 73301", false);
        let addr = out.address.unwrap();
        assert_eq!(
            addr.to_single_line(),
            "500 Elm St, Apt 4B, Austin, TX 73301"
        );
    }
}
