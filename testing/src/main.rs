// Generates exhaustive test cases.
// This was thrown together quite quickly to ensure the parser was correct.
// It could definitely be improved.

// Note that this could be made multi-threaded for a big speed up.
// Though it'll ideally only need to be generated once.

use std::{ffi::c_void, fs::File, os::windows::io::AsRawHandle, ptr::null_mut as null};

fn main() {
	println!("Generating permutations (this may take awhile)...");
	let mut buffer = Io::new("output.txt");
	
	// For the most part it should be sufficient to test a limited number of characters.
	let input: Vec<u16> = "\\a\" \t".encode_utf16().collect();

	// Uncomment this code if you don't mind waiting awhile.
	// Sample space: All ASCII characters (except `\0` and `\n`) and the characters `£` and `�`.
	//let input: Vec<u16> = "£�".encode_utf16().chain(1..=9).chain(0xB..=0x7f).collect();
	
	// Run `args.exe` with all the different combinations of characters as the
	// command line.
	// Adjust max_len as needed. Remember that the time taken increases exponentially. So adding
	// even one to the max_len can greatly increase the time taken.
	perms(&input, 6, move |perm| {
		run_args(perm, &mut buffer);
	});

	println!("Done.")
}

// Enumerate all permutations with repetitions and for all output lengths from 1 to `max_len`.
// Don't ask me how this works, I typed it out in a single stream of consciousness.
fn perms_iter<'a, T: Copy>(
	input: &'a [T],
	max_len: u32,
) -> impl Iterator<Item = impl Iterator<Item = T> + 'a> {
	(1..=max_len)
		.flat_map(move |len| (0..input.len().pow(len)).zip(std::iter::repeat(len)))
		.map(move |(mut n, j)| {
			(0..j).map(move |_| {
				let s = input[n % input.len()];
				n /= input.len();
				s
			})
		})
}
fn perms<F: FnMut(&mut [u16])>(input: &[u16], max_len: u16, mut f: F) {
	let mut buffer = Vec::with_capacity((max_len + 1) as _);

	for args in perms_iter(input, max_len as _) {
		buffer.clear();
		for unit in args {
			buffer.push(unit);
		}
		buffer.push(0);
		f(&mut buffer);
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

// Some thing to write the output to. Could be a pipe but in this case I'm saving directly to a file so it can be reused.
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
