use addrparts::address;

// Messy address strings shaped like what actually shows up in CSV exports
// and scraped pages, paired with whether they should parse as valid. Keeps
// regressions in whitespace/casing/PO-box handling from creeping back in
// without hand-writing a dedicated unit test for each one.
const FIXTURES: &str = include_str!("fixtures/messy_addresses.txt");

#[test]
fn messy_addresses_match_expected_validity() {
    let mut checked = 0;
    for (i, raw_line) in FIXTURES.lines().enumerate() {
        let line_no = i + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (expected, addr) = line.split_once('|').unwrap_or_else(|| {
            panic!("fixtures/messy_addresses.txt:{line_no}: missing '|' separator")
        });
        let want_valid = match expected {
            "valid" => true,
            "invalid" => false,
            other => panic!(
                "fixtures/messy_addresses.txt:{line_no}: expected 'valid' or 'invalid', got '{other}'"
            ),
        };

        let outcome = address::parse(addr, false);
        assert_eq!(
            outcome.is_valid(),
            want_valid,
            "fixtures/messy_addresses.txt:{line_no}: {addr:?} expected valid={want_valid}, got errors={:?}",
            outcome.errors
        );
        checked += 1;
    }

    assert!(checked > 0, "no fixtures were exercised");
}
