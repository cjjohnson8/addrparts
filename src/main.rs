mod address;

use address::ParseOutcome;
use std::env;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut json = false;
    let mut strict = false;
    let mut input_parts: Vec<String> = Vec::new();

    for arg in &args {
        match arg.as_str() {
            "--json" => json = true,
            "--strict" => strict = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::from(2);
            }
            other => input_parts.push(other.to_string()),
        }
    }

    let input = if input_parts.is_empty() {
        let mut buf = String::new();
        match io::stdin().read_to_string(&mut buf) {
            Ok(0) => {
                print_usage();
                return ExitCode::from(2);
            }
            Ok(_) => buf,
            Err(e) => {
                eprintln!("error reading stdin: {e}");
                return ExitCode::from(2);
            }
        }
    } else {
        input_parts.join(" ")
    };

    let outcome = address::parse(&input, strict);

    if json {
        println!("{}", to_json(&outcome));
    } else {
        print_human(&outcome);
    }

    if outcome.is_valid() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_usage() {
    eprintln!("addrparts - parse and validate a single-line US mailing address");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    addrparts [--json] [--strict] \"123 Main St, Springfield, IL 62704\"");
    eprintln!("    echo \"123 Main St, Springfield, IL 62704\" | addrparts [--json] [--strict]");
    eprintln!();
    eprintln!("Expected form: STREET, CITY, STATE ZIP[-ZIP4]");
    eprintln!("--strict requires the street to end in a standard USPS suffix");
    eprintln!("abbreviation (St, Ave, Blvd, ...) instead of a spelled-out word");
    eprintln!("Exit codes: 0 valid, 1 invalid, 2 usage error");
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
