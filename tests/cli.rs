// Exercises the compiled binary directly (argv, stdin, exit code, stdout vs.
// stderr) since none of that plumbing lives in the library and so isn't
// covered by src/address.rs's unit tests or tests/fixtures.rs.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_addrparts"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn addrparts binary");

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(stdin.as_bytes())
        .expect("failed to write to child stdin");

    child.wait_with_output().expect("child process failed")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout was not valid utf-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr was not valid utf-8")
}

#[test]
fn valid_address_arg_exits_zero() {
    let out = run(&["123 Main St, Springfield, IL 62704"], "");
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.contains("street: 123 Main St"));
    assert!(text.contains("valid:  true"));
}

#[test]
fn invalid_address_arg_exits_one() {
    let out = run(&["1 First Ave, Nowhere, ZZ 00000"], "");
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert!(text.contains("valid:  false"));
    assert!(text.contains("not a recognized state"));
}

#[test]
fn no_args_and_empty_stdin_exits_two_with_usage_on_stderr() {
    let out = run(&[], "");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("USAGE:"));
    assert!(stdout(&out).is_empty());
}

#[test]
fn help_flag_exits_two_with_usage_on_stderr() {
    let out = run(&["--help"], "");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("USAGE:"));
}

#[test]
fn json_flag_produces_single_object_with_escaped_quote() {
    let out = run(&["--json", "123 Main St, Spring \"field\", IL 62704"], "");
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.starts_with('{'));
    assert!(text.contains("\"valid\":true"));
    assert!(text.contains("\\\"field\\\""));
}

#[test]
fn format_flag_uppercases_state_but_preserves_other_casing() {
    let out = run(&["--format", "123 main st, springfield, il 62704"], "");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out), "123 main st, springfield, IL 62704\n");
}

#[test]
fn strict_flag_rejects_spelled_out_suffix() {
    let out = run(&["--strict", "123 Main Street, Springfield, IL 62704"], "");
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout(&out).contains("not a standard USPS street suffix"));
}

#[test]
fn stdin_batch_with_json_produces_array() {
    let input = "123 Main St, Springfield, IL 62704\n1 First Ave, Nowhere, ZZ 00000\n";
    let out = run(&["--json"], input);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert!(text.starts_with('['));
    assert!(text.trim_end().ends_with(']'));
    assert_eq!(text.matches("\"input\":").count(), 2);
}

#[test]
fn stdin_batch_human_output_separates_addresses_with_blank_line() {
    let input = "123 Main St, Springfield, IL 62704\n1600 Amphitheatre Pkwy, Mountain View, CA 94043\n";
    let out = run(&[], input);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.contains("Springfield"));
    assert!(text.contains("Mountain View"));
    assert!(text.contains("\n\n"));
}

#[test]
fn format_flag_skips_unparseable_line_with_stderr_message() {
    let out = run(&["--format", "just a string"], "");
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout(&out).is_empty());
    assert!(stderr(&out).contains("skipped (unparseable)"));
}
