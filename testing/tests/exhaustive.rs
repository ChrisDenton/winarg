use std::{
	fs::File,
	io::{self, BufRead},
	sync::atomic::{AtomicPtr, Ordering},
};

// Again, this isn't very good and should probably be rewritten.
// It does the job though (mostly).

#[test]
fn exhaustive() -> io::Result<()> {
	let f = File::open("output.txt")?;
	let reader = io::BufReader::new(f);
	let mut lines = reader.lines();
	let mut buffer = Vec::new();
	let mut counter = 0_usize;
	loop {
		let cmdline = match lines.next() {
			Some(line) => line?,
			None => break,
		};
		if cmdline.len() > 10 {
			break;
		}
		counter += 1;
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
		//
		let cmd: Vec<u16> = cmdline.encode_utf16().chain(Some(0)).collect();
		CMD.store(cmd.as_ptr() as _, std::sync::atomic::Ordering::Relaxed);

		let mut counter = 0;
		for arg in winarg::ArgsNative::from_env() {
			assert_eq!(arg, buffer[counter].as_str());
			counter += 1;
		}
		assert_eq!(counter, buffer.len());

		let mut counter = 0;
		let args: String = winarg::null_separated_list().collect();
		for arg in args.split('\0') {
			assert_eq!(arg, buffer[counter], "{:?}", cmdline);
			counter += 1;
		}
		assert_eq!(counter, buffer.len());

		let mut parser = winarg::Parser();
		// Skip the zeroth argument.
		for t in &mut parser {
			if t.is_next_arg() {
				break;
			}
		}

		// Collect the rest into a UTF-16 encoded vector.
		let args: Vec<u16> = parser.map(|t| t.as_u16()).collect();
		for (index, arg) in args.split(|&w| w == 0).enumerate() {
			// `split` will produce a single empty argument if args is empty.
			if index + 1 == buffer.len() {
				assert!(arg.is_empty());
			} else {
				let arg = String::from_utf16_lossy(arg);
				assert_eq!(arg, buffer[index + 1], "{:?}", cmdline);
			}
		}
	}
	println!("{}", counter);
	Ok(())
}

// Replace `GetCommandLineW` with our own version.
// WARNING: Never do this in real code.
// Replacing system functions is massively unsafe and a recipe for disaster.
// Especially so with a library that assumes the memory is 'static.
static CMD: AtomicPtr<u16> = AtomicPtr::new(std::ptr::null_mut());
#[no_mangle]
extern "C" fn GetCommandLineW() -> *const u16 {
	CMD.load(Ordering::Relaxed)
}
