// Writes test cases to stdout in a form suitable for the standard library.
// This uses raw strings where necessary so that the test cases are easier for humans to read.
// Warning: If any test case includes the `#` character then this code may need
// to be updated to escape it properly in a Rust string literal.

use std::fs;
use std::io::{self, BufRead};

fn main() -> io::Result<()> {
	let f = fs::File::open("test Cases")?;
	let reader = io::BufReader::new(f);
	let mut lines = reader.lines();
	let mut buffer = Vec::new();
	
	loop {
		let cmdline = match lines.next() {
			Some(line) => line?,
			None => break,
		};
		let argc = match lines.next() {
			Some(line) => line?.parse::<u16>().unwrap(),
			None => break,
		};
		buffer.clear();
		for _ in 0..argc {
			let arg = match lines.next() {
				Some(line) => line?,
				None => break,
			};
			buffer.push(arg);
		}
		if cmdline.contains(|c| c=='"' || c=='\\') {
			print!(r##"chk(r#"{}"#, &["##, cmdline);
		} else {
			print!(r#"chk("{}", &["#, cmdline);
		}
		for arg in &buffer {
			if arg.contains('"') {
				print!(r##"r#"{}"#, "##, arg);
			} else if arg.contains('\\') {
				print!(r#"r"{}", "#, arg);
			} else {
				print!(r#""{}", "#, arg);
			}
		}
		println!("]);");
	}
	Ok(())
}