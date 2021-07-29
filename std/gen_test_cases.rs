use std::{ffi::c_void, fs::File, os::windows::io::AsRawHandle, ptr::null_mut as null};

fn main() {
	// This uses explicitly written tests. The main branch has a framework for more extensive testing.
	let tests = [
		// single_words
		"EXE one_word",
		"EXE a",
		"EXE 😅",
		"EXE 😅🤦",

		// official_examples
		r#"EXE "abc" d e"#,
		r#"EXE a\\\b d"e f"g h"#,
		r#"EXE a\\\"b c d"#,
		r#"EXE a\\\\"b c" d e"#,

		// whitespace_behavior
		r#" test"#,
		r#"  test"#,
		r#" test test2"#,
		r#" test  test2"#,
		r#"test test2 "#,
		r#"test  test2 "#,
		r#"test "#,

		// genius_quotes
		r#"EXE "" """#,
		r#"EXE "" """"#,
		r#"EXE "this is """all""" in the same argument""#,
		r#"EXE "a"""#,
		r#"EXE "a"" a"#,
		r#""EXE" check"#,
		r#""EXE check""#,
		r#""EXE """for""" check"#,
		r#""EXE \"for\" check"#,
		r#""EXE \" for \" check"#,
		r#"E"X"E test"#,
		r#"EX""E test"#,

		// from https://daviddeley.com/autohotkey/parameters/parameters.htm#WINCRULESEX
		r#"EXE CallMeIshmael"#,
		r#"EXE "Call Me Ishmael""#,
		r#"EXE Cal"l Me I"shmael"#,
		r#"EXE CallMe\"Ishmael"#,
		r#"EXE "CallMe\"Ishmael""#,
		r#"EXE "Call Me Ishmael\\""#,
		r#"EXE "CallMe\\\"Ishmael""#,
		r#"EXE a\\\b"#,
		r#"EXE "a\\\b""#,

		r#"EXE "\"Call Me Ishmael\"""#,
		r#"EXE "C:\TEST A\\""#,
		r#"EXE "\"C:\TEST A\\\"""#,

		r#"EXE "a b c"  d  e"#,
		r#"EXE "ab\"c"  "\\"  d"#,
		r#"EXE a\\\b d"e f"g h"#,
		r#"EXE a\\\"b c d"#,
		r#"EXE a\\\\"b c" d e"#,

		r#"EXE "a b c"""#,
		r#"EXE """CallMeIshmael"""  b  c"#,
		r#"EXE """Call Me Ishmael""""#,
		r#"EXE """"Call Me Ishmael"" b c"#,
	];

	{
		let mut buffer = Io::new("test Cases");
		for test in tests {
			let mut cmd: Vec<u16> = test.encode_utf16().chain(Some(0)).collect();
			run_args(&mut cmd, &mut buffer);
		}
	}
}

// Call `CreateProcessW` with the command line and write the output to `buffer`.
// We can't use std::process::Command because it doesn't (yet) allow setting the
// zeroth argument. Nightly does support raw_args for setting the others.
fn run_args(args: &mut [u16], buffer: &mut Io) {
	// args.exe
	static NAME: &[u16] = &[
		b'a' as _, b'r' as _, b'g' as _, b's' as _, b'.' as _, b'e' as _, b'x' as _, b'e' as _, 0,
	];
	unsafe {
		let mut startup = STARTUPINFOW::new();
		startup.dwFlags = 0x100;
		startup.hStdOutput = buffer.write;

		let mut info = PROCESS_INFORMATION::new();
		let result = CreateProcessW(
			NAME.as_ptr(),
			args.as_mut_ptr(),
			null(),
			null(),
			1,
			0,
			null(),
			null(),
			&startup,
			&mut info,
		);
		assert_ne!(result, 0);
		// Wait for the process to exit to avoid overlapping writes.
		let result = WaitForSingleObject(info.hProcess, u32::MAX);
		assert_ne!(result, u32::MAX);

		let mut exit_code = 0;
		let result = GetExitCodeProcess(info.hProcess, &mut exit_code);
		assert_ne!(result, 0);
		assert_eq!(exit_code, 0);

		CloseHandle(info.hProcess);
		CloseHandle(info.hThread);
	}
}

// Some thing to write the output to. Could be a pipe but in this case I'm
// saving directly to a file so it can be reused without have to regenerate
// every time (which can be slow when doing more exhaustive testing).
struct Io {
	write: usize,
}
impl Io {
	fn new(name: &str) -> Self {
		let f = File::create(name).unwrap();
		let handle = f.as_raw_handle();
		let mut output = 0;
		unsafe {
			DuplicateHandle(
				GetCurrentProcess(),
				handle as _,
				GetCurrentProcess(),
				&mut output,
				0,
				1,
				2,
			);
		}
		Self { write: output }
	}
}
impl Drop for Io {
	fn drop(&mut self) {
		unsafe {
			CloseHandle(self.write);
		}
	}
}

#[repr(C)]
#[allow(nonstandard_style)]
struct PROCESS_INFORMATION {
	hProcess: usize,
	hThread: usize,
	dwProcessId: u32,
	dwThreadId: u32,
}
impl PROCESS_INFORMATION {
	fn new() -> Self {
		unsafe { return std::mem::zeroed() }
	}
}
#[repr(C)]
#[allow(nonstandard_style)]
struct STARTUPINFOW {
	cb: u32,
	lpReserved: *mut u16,
	lpDesktop: *mut u16,
	lpTitle: *mut u16,
	dwX: u32,
	dwY: u32,
	dwXSize: u32,
	dwYSize: u32,
	dwXCountChars: u32,
	dwYCountChars: u32,
	dwFillAttribute: u32,
	dwFlags: u32,
	wShowWindow: u16,
	cbReserved2: u16,
	lpReserved2: *mut u8,
	hStdInput: usize,
	hStdOutput: usize,
	hStdError: usize,
}
impl STARTUPINFOW {
	fn new() -> Self {
		let mut new: STARTUPINFOW = unsafe { std::mem::zeroed() };
		new.cb = std::mem::size_of::<Self>() as _;
		new
	}
}

extern "system" {
	fn CreateProcessW(
		lpApplicationName: *const u16,
		lpCommandLine: *mut u16,
		lpProcessAttributes: *const c_void,
		lpThreadAttributes: *const c_void,
		bInheritHandles: i32,
		dwCreationFlags: u32,
		lpEnvironment: *const u16,
		lpCurrentDirectory: *const u16,
		lpStartupInfo: *const STARTUPINFOW,
		lpProcessInformation: *mut PROCESS_INFORMATION,
	) -> i32;
	fn WaitForSingleObject(hHandle: usize, dwMilliseconds: u32) -> u32;
	fn CloseHandle(hObject: usize) -> i32;
	fn DuplicateHandle(
		hSourceProcessHandle: usize,
		hSourceHandle: usize,
		hTargetProcessHandle: usize,
		lpTargetHandle: *mut usize,
		dwDesiredAccess: u32,
		bInheritHandle: i32,
		dwOptions: u32,
	) -> i32;
	fn GetCurrentProcess() -> usize;
	fn GetExitCodeProcess(hProcess: usize, lpExitCode: *mut u32) -> i32;
}
