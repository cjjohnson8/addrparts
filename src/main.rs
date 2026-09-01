mod address;

use address::ParseOutcome;
use std::env;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut json = false;
    let mut strict = false;
    let mut format = false;
    let mut input_parts: Vec<String> = Vec::new();

    for arg in &args {
        match arg.as_str() {
            "--json" => json = true,
            "--strict" => strict = true,
            "--format" => format = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::from(2);
            }
            other => input_parts.push(other.to_string()),
        }
    }

    if !input_parts.is_empty() {
        let outcome = address::parse(&input_parts.join(" "), strict);
        if format {
            print_format(&outcome);
        } else if json {
            println!("{}", to_json(&outcome));
        } else {
            print_human(&outcome);
        }
        return if outcome.is_valid() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }

    let mut buf = String::new();
    match io::stdin().read_to_string(&mut buf) {
        Ok(0) => {
            print_usage();
            return ExitCode::from(2);
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("error reading stdin: {e}");
            return ExitCode::from(2);
        }
    }

    let outcomes = address::parse_lines(&buf, strict);
    if outcomes.is_empty() {
        print_usage();
        return ExitCode::from(2);
    }

    let all_valid = outcomes.iter().all(ParseOutcome::is_valid);

    if outcomes.len() == 1 {
        if format {
            print_format(&outcomes[0]);
        } else if json {
            println!("{}", to_json(&outcomes[0]));
        } else {
            print_human(&outcomes[0]);
        }
    } else if format {
        for outcome in &outcomes {
            print_format(outcome);
        }
    } else if json {
        let items: Vec<String> = outcomes.iter().map(to_json).collect();
        println!("[{}]", items.join(","));
    } else {
        for (i, outcome) in outcomes.iter().enumerate() {
            if i > 0 {
                println!();
            }
            print_human(outcome);
        }
    }

    if all_valid {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_usage() {
    eprintln!("addrparts - parse and validate a single-line US mailing address");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    addrparts [--json] [--strict] [--format] \"123 Main St, Springfield, IL 62704\"");
    eprintln!("    echo \"123 Main St, Springfield, IL 62704\" | addrparts [--json] [--strict] [--format]");
    eprintln!();
    eprintln!("Stdin may contain multiple addresses, one per line. Blank lines are");
    eprintln!("skipped. With more than one address, --json emits a JSON array instead");
    eprintln!("of a single object, and human output separates addresses with a blank line.");
    eprintln!();
    eprintln!("Expected form: STREET, CITY, STATE ZIP[-ZIP4]");
    eprintln!("--strict requires the street to end in a standard USPS suffix");
    eprintln!("abbreviation (St, Ave, Blvd, ...) instead of a spelled-out word");
    eprintln!();
    eprintln!("--format prints the parsed fields rejoined into a single line");
    eprintln!("(state upper-cased) instead of the usual report, one line per");
    eprintln!("address. Addresses that couldn't be split into fields at all are");
    eprintln!("skipped with a message on stderr. Takes precedence over --json.");
    eprintln!();
    eprintln!("Exit codes: 0 valid, 1 invalid, 2 usage error");
}

// Prints the reassembled single-line form of an address that was split
// far enough to have street/city/state/zip fields, even if it failed
// other validation (e.g. an unrecognized state code). Addresses that
// couldn't be split at all have nothing to reassemble.
fn print_format(outcome: &ParseOutcome) {
    match &outcome.address {
        Some(addr) => println!("{}", addr.to_single_line()),
        None => eprintln!("skipped (unparseable): {}", outcome.input),
    }
}

fn print_human(outcome: &ParseOutcome) {
    match &outcome.address {
        Some(addr) => {
            println!("street: {}", addr.street);
            println!("city:   {}", addr.city);
            println!("state:  {}", addr.state);
            match &addr.zip4 {
                Some(z4) => println!("zip:    {}-{}", addr.zip5, z4),
                None => println!("zip:    {}", addr.zip5),
            }
            if let Some(po_box) = &addr.po_box {
                println!("po box: {po_box}");
            }
        }
        None => println!("(no fields could be parsed)"),
    }

    if outcome.errors.is_empty() {
        println!("valid:  true");
    } else {
        println!("valid:  false");
        for e in &outcome.errors {
            println!("error:  {e}");
        }
    }
}

fn to_json(outcome: &ParseOutcome) -> String {
    let mut out = String::from("{");
    out.push_str(&format!("\"input\":{}", json_string(&outcome.input)));
    out.push_str(&format!(",\"valid\":{}", outcome.is_valid()));

    match &outcome.address {
        Some(addr) => {
            out.push_str(",\"address\":{");
            out.push_str(&format!("\"street\":{}", json_string(&addr.street)));
            out.push_str(&format!(",\"city\":{}", json_string(&addr.city)));
            out.push_str(&format!(",\"state\":{}", json_string(&addr.state)));
            out.push_str(&format!(",\"zip5\":{}", json_string(&addr.zip5)));
            match &addr.zip4 {
                Some(z4) => out.push_str(&format!(",\"zip4\":{}", json_string(z4))),
                None => out.push_str(",\"zip4\":null"),
            }
            match &addr.po_box {
                Some(po_box) => out.push_str(&format!(",\"po_box\":{}", json_string(po_box))),
                None => out.push_str(",\"po_box\":null"),
            }
            out.push('}');
        }
        None => out.push_str(",\"address\":null"),
    }

    out.push_str(",\"errors\":[");
    let escaped: Vec<String> = outcome.errors.iter().map(|e| json_string(e)).collect();
    out.push_str(&escaped.join(","));
    out.push(']');

    out.push('}');
    out
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
