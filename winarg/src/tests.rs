// Currently this tests the parsing but not the API.
// Doc tests do some limited testing of the API but it'd be good to have something more thorough.
// This situation should be improved before 1.0.

extern crate alloc;
use super::{scalars, Parser};
use alloc::{string::String, vec::Vec};

/*-*-*-*-*

# Parsing tests

These tests are intended to eventually be the same in the standard library (PR pending).
They are based on the original standard library tests but updated for modern C/C++ behaviour
and with additions from https://daviddeley.com/autohotkey/parameters/parameters.htm#WINCRULESEX

*-*-*-*-*/

// Currently this library always uses `GetCommandLineW` so for testing we need
// a new function that uses the parser.
fn null_separated_list(cmdline: &[u16]) -> String {
	unsafe {
		// Note: `from_ptr` is not public and *probably* never will be.
		// However, it might make sense to have a public function that operates on a slice.
		scalars(Parser::from_ptr(cmdline.as_ptr()).map(|t| t.as_u16())).collect()
	}
}

fn chk(string: &str, parts: &[&str]) {
	let cmdline: Vec<u16> = string.encode_utf16().chain(Some(0)).collect();
	let args = null_separated_list(&cmdline);
	let mut len = 0;
	for (arg, &part) in args.split('\0').zip(parts) {
		assert_eq!(arg, part);
		len += 1;
	}
	assert_eq!(len, parts.len());
}

#[test]
fn single_words() {
	chk("EXE one_word", &["EXE", "one_word"]);
	chk("EXE a", &["EXE", "a"]);
	chk("EXE 😅", &["EXE", "😅"]);
	chk("EXE 😅🤦", &["EXE", "😅🤦"]);
}

#[test]
fn official_examples() {
	chk(r#"EXE "abc" d e"#, &["EXE", "abc", "d", "e"]);
	chk(r#"EXE a\\\b d"e f"g h"#, &["EXE", r"a\\\b", "de fg", "h"]);
	chk(r#"EXE a\\\"b c d"#, &["EXE", r#"a\"b"#, "c", "d"]);
	chk(r#"EXE a\\\\"b c" d e"#, &["EXE", r"a\\b c", "d", "e"]);
}

#[test]
fn whitespace_behavior() {
	chk(" test", &["", "test"]);
	chk("  test", &["", "test"]);
	chk(" test test2", &["", "test", "test2"]);
	chk(" test  test2", &["", "test", "test2"]);
	chk("test test2 ", &["test", "test2"]);
	chk("test  test2 ", &["test", "test2"]);
	chk("test ", &["test"]);
}

#[test]
fn genius_quotes() {
	chk(r#"EXE "" """#, &["EXE", "", ""]);
	chk(r#"EXE "" """"#, &["EXE", "", r#"""#]);
	chk(
		r#"EXE "this is """all""" in the same argument""#,
		&["EXE", r#"this is "all" in the same argument"#],
	);
	chk(r#"EXE "a"""#, &["EXE", r#"a""#]);
	chk(r#"EXE "a"" a"#, &["EXE", r#"a" a"#]);
	// quotes cannot be escaped in command names
	chk(r#""EXE" check"#, &["EXE", "check"]);
	chk(r#""EXE check""#, &["EXE check"]);
	chk(r#""EXE """for""" check"#, &["EXE for check"]);
	chk(r#""EXE \"for\" check"#, &[r"EXE \for\ check"]);
	chk(
		r#""EXE \" for \" check"#,
		&[r"EXE \", "for", r#"""#, "check"],
	);
	chk(r#"E"X"E test"#, &["EXE", "test"]);
	chk(r#"EX""E test"#, &["EXE", "test"]);
}

// from https://daviddeley.com/autohotkey/parameters/parameters.htm#WINCRULESEX
#[test]
fn post_2008() {
	chk("EXE CallMeIshmael", &["EXE", "CallMeIshmael"]);
	chk(r#"EXE "Call Me Ishmael""#, &["EXE", "Call Me Ishmael"]);
	chk(r#"EXE Cal"l Me I"shmael"#, &["EXE", "Call Me Ishmael"]);
	chk(r#"EXE CallMe\"Ishmael"#, &["EXE", r#"CallMe"Ishmael"#]);
	chk(r#"EXE "CallMe\"Ishmael""#, &["EXE", r#"CallMe"Ishmael"#]);
	chk(r#"EXE "Call Me Ishmael\\""#, &["EXE", r"Call Me Ishmael\"]);
	chk(r#"EXE "CallMe\\\"Ishmael""#, &["EXE", r#"CallMe\"Ishmael"#]);
	chk(r#"EXE a\\\b"#, &["EXE", r"a\\\b"]);
	chk(r#"EXE "a\\\b""#, &["EXE", r"a\\\b"]);
	chk(
		r#"EXE "\"Call Me Ishmael\"""#,
		&["EXE", r#""Call Me Ishmael""#],
	);
	chk(r#"EXE "C:\TEST A\\""#, &["EXE", r"C:\TEST A\"]);
	chk(r#"EXE "\"C:\TEST A\\\"""#, &["EXE", r#""C:\TEST A\""#]);
	chk(r#"EXE "a b c"  d  e"#, &["EXE", "a b c", "d", "e"]);
	chk(r#"EXE "ab\"c"  "\\"  d"#, &["EXE", r#"ab"c"#, r"\", "d"]);
	chk(r#"EXE a\\\b d"e f"g h"#, &["EXE", r"a\\\b", "de fg", "h"]);
	chk(r#"EXE a\\\"b c d"#, &["EXE", r#"a\"b"#, "c", "d"]);
	chk(r#"EXE a\\\\"b c" d e"#, &["EXE", r"a\\b c", "d", "e"]);
	// Double Double Quotes
	chk(r#"EXE "a b c"""#, &["EXE", r#"a b c""#]);
	chk(
		r#"EXE """CallMeIshmael"""  b  c"#,
		&["EXE", r#""CallMeIshmael""#, "b", "c"],
	);
	chk(
		r#"EXE """Call Me Ishmael""""#,
		&["EXE", r#""Call Me Ishmael""#],
	);
	chk(
		r#"EXE """"Call Me Ishmael"" b c"#,
		&["EXE", r#""Call"#, "Me", "Ishmael", "b", "c"],
	);
}
