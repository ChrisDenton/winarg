#![no_std]
#![cfg(windows)]
#![cfg_attr(docsrs, feature(doc_cfg))]
//! The Windows command line is passed to applications as a string. To get an
//! array of arguments it's necessary to parse this string, which is what this
//! crate does. This list of arguments can then be used by higher level argument
//! parsers.
//!
//! It uses the latest C/C++ parsing rules so that it is consistent with using
//! `argv` from a C/C++ program.
//!
//! # Using
//!
//! Add this to your `Cargo.toml` file
//!
//! ```ini
//! [dependencies.winarg]
//! version = "0.2.0"
//! ```
//!
//! # Examples
//!
//! Iterating arguments without allocation using [`args_native`]:
//!
//! ```
//! for arg in winarg::args_native().skip(1) {
//!     if arg == "--help" {
//!         println!("help me!");
//!     }
//! }
//! ```
//!
//! Creating a single `String` buffer to hold a list of arguments using [`null_separated_list`]:
//!
//! ```
//! let args: String = winarg::null_separated_list().collect();
//! for arg in args.split('\0') {
//!     println!("{}", arg);
//! }
//! ```
//!
//! [`struct@Parser`] can be used to create custom constructs. For example:
//!
//! ```
//! let mut parser = winarg::Parser();
//! // Skip the zeroth argument.
//! for t in &mut parser {
//!     if t.is_next_arg() { break; }
//! }
//! // Collect the rest into a UTF-16 encoded vector.
//! let args: Vec<u16> = parser.map(|t| t.as_u16() ).collect();
//! ```
//!
//! # Features
//!
//! - `std`: Adds the error trait to [`NoNullError`].

/*
Implementation note: The public interface and the private implementation were
basically created separately then glued together. I've started joining them up
more but there's still a lot artificial separation and indirection.
*/

#[cfg(test)]
mod tests;

use core::{
	char::{decode_utf16, REPLACEMENT_CHARACTER},
	fmt::{self, Display},
	marker::PhantomData,
	num::NonZeroU16,
	slice,
};
#[cfg(feature = "std")]
extern crate std;

const SPACE: u16 = b' ' as _;
const TAB: u16 = b'\t' as _;
const QUOTE: u16 = b'"' as _;
const SLASH: u16 = b'\\' as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
	/// A UTF-16 code unit.
	Unit(NonZeroU16),
	/// The end of the previous argument and the beginning of a new argument.
	/// Note that [`struct@Parser`] will *not* return this after the final argument.
	NextArg,
}
impl Token {
	#[inline]
	/// Convert the token to a `u16` value. [`Token::NextArg`] will be converted to `0`.
	pub fn as_u16(self) -> u16 {
		match self {
			Token::Unit(u) => u.get(),
			Token::NextArg => 0,
		}
	}
	/// Helper to test for [`Token::NextArg`].
	#[inline]
	pub fn is_next_arg(self) -> bool {
		self == Token::NextArg
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct NoNullError;
impl Display for NoNullError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("wide string is not null terminated")
	}
}
#[cfg(feature = "std")]
impl std::error::Error for NoNullError {}

/// A parsing iterator that produces [`Token`]s.
///
/// Can be use to build your own higher level constructs. For example,
/// it can be used in place of [`null_separated_list_wide`] like so:
///
/// ```
/// let args: Vec<u16> = winarg::Parser().map(|t| t.as_u16() ).collect();
/// ```
#[derive(Debug, Clone)]
pub struct Parser<'a> {
	iter: ParseArgs<'a>,
}
impl Parser<'_> {
	pub fn from_env() -> Parser<'static> {
		Parser()
	}

	/// Create from a slice.
	///
	/// Currently the `command_line` must be null terminated or else it's an error.
	/// This may change in a future version.
	///
	/// While it is not an error for `command_line` to contain interior nulls,
	/// it will result in the arguments being truncated at the first null.
	pub fn from_slice(command_line: &[u16]) -> Result<Parser<'_>, NoNullError> {
		if command_line.last() == Some(&0) {
			// We've ensure that the &[u16] is null terminated.
			// NOTE: it may still contain interior nulls.
			// This is memory safe even though it's logically wrong.
			unsafe { Ok(Parser::from_ptr(command_line.as_ptr())) }
		} else {
			Err(NoNullError)
		}
	}
	/// SAFETY: `lpwstr` must point to valid null terminated wide string.
	/// It must live at least as long as this struct and must not be modified.
	unsafe fn from_ptr(lpwstr: *const u16) -> Self {
		Self {
			iter: ParseArgs::from_ptr(lpwstr),
		}
	}
}
impl Iterator for Parser<'_> {
	type Item = Token;
	fn next(&mut self) -> Option<Self::Item> {
		self.iter
			.next()
			// SAFETY: ParseArgs will never return zero as it terminates the command line.
			.map(|w| unsafe { Token::Unit(NonZeroU16::new_unchecked(w)) })
			.or_else(|| {
				self.iter.move_to_next_arg();
				if self.iter.cursor.peek().is_none() {
					None
				} else {
					Some(Token::NextArg)
				}
			})
	}
	/// Calculate the maximum possible size by scanning for the terminating null.
	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.iter.cursor.max_len() as _))
	}
}

#[allow(nonstandard_style)]
#[doc(hidden)]
pub fn Parser() -> Parser<'static> {
	Parser {
		iter: ParseArgs::from_env(),
	}
}

/// A list of arguments separated by a `\0` character.
/// ```
/// let args: String = winarg::null_separated_list().collect();
/// for arg in args.split('\0') {
///     println!("{}", arg);
/// }
/// ```
pub fn null_separated_list() -> impl Iterator<Item = char> + fmt::Debug + Clone {
	scalars(null_separated_list_wide())
}
/// A list of UTF-16 encoded arguments, separated by a NULL.
/// ```
/// let args: Vec<u16> = winarg::null_separated_list_wide().collect();
/// for arg in args.split(|&w| w == 0) {
///     println!("{}", String::from_utf16_lossy(arg));
/// }
/// ```
pub fn null_separated_list_wide() -> impl Iterator<Item = u16> + fmt::Debug + Clone {
	Parser().map(|t| t.as_u16())
}

/// A command line argument.
///
/// Can be encoded as scalars, code points or UTF-16 code units.
/// Arguments can also be compared to `&str` or `&[u16]` slices.
///
/// ```
/// if let Some(arg) = winarg::ArgsNative::from_env().next() {
///     if arg == "binname.exe" {
///         println!("Program was called as `binname.exe`");
///     }
/// }
/// ```
#[derive(Clone)]
pub struct Argument<'a> {
	arg: WideIter<'a>,
	is_arg0: bool,
}
impl<'a> Argument<'a> {
	/// Iterates scalar values. Isolated surrogates will be replaced with
	/// the replacement character (`�`).
	///
	/// ```
	/// for arg in winarg::args_native() {
	///     let arg: String = arg.scalars().collect();
	///     println!("{}", arg);
	/// }
	/// ```
	pub fn scalars(&self) -> impl Iterator<Item = char> + fmt::Debug + Clone + 'a {
		scalars(self.utf16_units())
	}
	/// Iterates code points. These are similar to scalar values except that
	/// they may contain isolated surrogates.
	///
	/// ```
	/// use std::char;
	///
	/// for arg in winarg::args_native() {
	///     let arg: String = arg.code_points()
	///     .map(|cp| char::from_u32(cp)
	///     .unwrap_or(char::REPLACEMENT_CHARACTER))
	///     .collect();
	///     println!("{}", arg);
	/// }
	/// ```
	pub fn code_points(&self) -> impl Iterator<Item = u32> + fmt::Debug + Clone + 'a {
		code_points(self.utf16_units())
	}

	/// Iterates UTF-16 code units. May contain isolated surrogates, which means it's invalid Unicode.
	///
	/// ```
	/// use std::os::windows::ffi::OsStringExt;
	/// use std::ffi::OsString;
	///
	/// for arg in winarg::args_native() {
	///     let arg: Vec<u16> = arg.utf16_units().collect();
	///     let arg = OsString::from_wide(&arg);
	///     println!("{:?}", arg);
	/// }
	/// ```
	pub fn utf16_units(&self) -> impl Iterator<Item = u16> + fmt::Debug + Clone + 'a {
		ParseArgs::new(self.arg, self.is_arg0)
	}

	/// Get the rest of the command line as a single, unparsed, argument. This
	/// may contain quotes and escape characters.
	///
	/// While this is rarely used, it can be useful for passing on arguments to
	/// other programs or doing non-standard parsing.
	///
	/// ```
	/// let mut args = winarg::args_native();
	/// let mut raw_args = None;
	/// while let Some(arg) = args.next() {
	///     if arg == "--" {
	///          // Gets the next argument and call `raw_arg`.
	///          raw_args = args.next().map(|arg| arg.raw_arg());
	///          // Stop parsing the arguments.
	///          break;
	///     }
	/// }
	/// ```
	pub fn raw_arg(&self) -> &'static [u16] {
		// SAFETY: `GetCommandLineW`'s memory is never freed for the lifetime of the process.
		unsafe { self.arg.as_slice() }
	}

	fn eq<I: Iterator<Item = u16>>(&self, other: I) -> bool {
		self.utf16_units().eq(other)
	}
}
impl fmt::Debug for Argument<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Argument")
			.field("arg", &self.arg.ptr)
			.field("is_arg0", &self.is_arg0)
			.finish()
	}
}
impl Eq for Argument<'_> {}
impl PartialEq<Argument<'_>> for Argument<'_> {
	fn eq(&self, other: &Argument) -> bool {
		self.eq(other.utf16_units())
	}
}
impl PartialEq<Argument<'_>> for &str {
	fn eq(&self, other: &Argument) -> bool {
		other.eq(self.encode_utf16())
	}
}
impl PartialEq<&str> for Argument<'_> {
	fn eq(&self, other: &&str) -> bool {
		self.eq(other.encode_utf16())
	}
}
impl PartialEq<&[u16]> for Argument<'_> {
	fn eq(&self, other: &&[u16]) -> bool {
		self.eq(other.iter().copied())
	}
}
impl PartialEq<Argument<'_>> for &[u16] {
	fn eq(&self, other: &Argument) -> bool {
		other.eq(self.iter().copied())
	}
}

/// An iterator over native command line [`Argument`]s.
pub struct ArgsNative<'a> {
	next: ParseArgs<'a>,
}
impl ArgsNative<'_> {
	/// Get the command line arguments from the environment.
	///
	/// ```
	/// use winarg::ArgsNative;
	///
	/// for arg in ArgsNative::from_env() {
	///     let arg: String = arg.scalars().collect();
	///     println!("{}", arg);
	/// }
	/// ```
	pub fn from_env() -> ArgsNative<'static> {
		let arg = ParseArgs::from_env();
		ArgsNative { next: arg }
	}
}
impl fmt::Debug for ArgsNative<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ArgsNative")
			.field("next_arg", &self.next.cursor.ptr)
			.field("is_arg0", &self.next.is_arg0)
			.finish()
	}
}
impl<'a> Iterator for ArgsNative<'a> {
	type Item = Argument<'a>;
	fn next(&mut self) -> Option<Self::Item> {
		let current = Argument {
			arg: self.next.cursor,
			is_arg0: self.next.is_arg0,
		};
		if current.arg.peek().is_none() {
			None
		} else {
			self.next.move_to_next_arg();
			Some(current)
		}
	}
}

/// An iterator over the program's command line [`Argument`]s
///
/// ```
/// for arg in winarg::args_native() {
///     let arg: String = arg.scalars().collect();
///     println!("{}", arg);
/// }
/// ```
pub fn args_native<'a>() -> ArgsNative<'a> {
	ArgsNative::from_env()
}

/// Simple iterator to encapsulate the unsafety inherent in using a null terminated array without a length.
#[derive(Copy, Clone, Debug)]
struct WideIter<'a> {
	ptr: *const u16,
	lifetime: PhantomData<&'a [u16]>,
}
impl WideIter<'_> {
	/// # SAFETY
	/// * `ptr` must point to a NULL terminated `u16` array.
	/// * The array pointed to by `ptr` must exist for the lifetime of this struct.
	unsafe fn new(ptr: *const u16) -> Self {
		Self {
			ptr,
			lifetime: PhantomData,
		}
	}
	fn next(&mut self) -> Option<u16> {
		// SAFETY: The call to `peek` makes sure we haven't reached the NULL yet.
		// Therefore it's safe to advance the pointer.
		unsafe {
			let next = self.peek()?;
			self.ptr = self.ptr.add(1);
			Some(next)
		}
	}
	fn peek(&self) -> Option<u16> {
		// SAFETY: It's always safe to read the current item because we don't
		// ever move out of the array bounds.
		match unsafe { *self.ptr } {
			0 => None,
			next => Some(next),
		}
	}
	// SAFETY: The lifetime of the slice cannot outlive the lifetime of the memory.
	// This is not a problem for 'static memory.
	unsafe fn as_slice<'a>(self) -> &'a [u16] {
		let mut len = 0;
		while *self.ptr.add(len) != 0 {
			len += 1;
		}
		slice::from_raw_parts(self.ptr, len)
	}

	fn max_len(mut self) -> u16 {
		let mut len: u16 = 0;
		while self.next().is_some() {
			len += 1;
		}
		len
	}

	fn skip_whitespace(&mut self) {
		while self.peek() == Some(SPACE) || self.peek() == Some(TAB) {
			self.next();
		}
	}
}
// TODO: Don't implement iterator?
impl Iterator for WideIter<'_> {
	type Item = u16;
	fn next(&mut self) -> Option<Self::Item> {
		self.next()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EscapeMode {
	Unescaped,
	LiteralQuote,
}
#[derive(Clone, Debug)]
struct EscapeIter {
	counter: u16,
	mode: EscapeMode,
}
impl EscapeIter {
	// Count the number of consecutive slashes and check if it ends with a quote.
	fn new(iter: &mut WideIter) -> Self {
		let mut counter: u16 = 1;
		let mut mode = EscapeMode::Unescaped;
		loop {
			match iter.peek() {
				Some(SLASH) => counter += 1,
				Some(QUOTE) => {
					// If the counter is odd then output a literal quote instead
					// of toggling quote mode.
					if is_odd(counter) {
						iter.next();
						mode = EscapeMode::LiteralQuote;
					}
					// Ignore half of the slashes.
					counter /= 2;
					break;
				}
				// Don't escape anything. The slashes will be output literally.
				_ => break,
			}
			iter.next();
		}
		Self { counter, mode }
	}
	// Drain the saved state.
	fn next(&mut self) -> Option<u16> {
		match self.counter.checked_sub(1) {
			Some(n) => {
				self.counter = n;
				Some(SLASH)
			}
			None if self.mode == EscapeMode::LiteralQuote => {
				self.mode = EscapeMode::Unescaped;
				Some(QUOTE)
			}
			None => None,
		}
	}
}
impl Iterator for EscapeIter {
	type Item = u16;

	fn next(&mut self) -> Option<Self::Item> {
		self.next()
	}
}

/// An iterator over the UTF-16 code units of an argument
///
/// It can also be used to move forward to the next argument.
///
/// This should be considered unstable because it's essentially leaking an
/// implementation detail.
#[derive(Clone, Debug)]
struct ParseArgs<'a> {
	cursor: WideIter<'a>,
	quote_mode: bool,
	escape_iter: Option<EscapeIter>,
	is_arg0: bool,
}
impl ParseArgs<'_> {
	/// Creates an `ArgIter` from the environment, starting at the zeroth
	/// argument.
	fn from_env() -> ParseArgs<'static> {
		Self::new(command_line(), true)
	}
	/// SAFETY: `lpwstr` must point to valid null terminated wide string.
	/// It must live at least as long as this struct and must not be modified.
	unsafe fn from_ptr(lpwstr: *const u16) -> Self {
		Self::new(WideIter::new(lpwstr), true)
	}
	fn new(arg: WideIter, is_arg0: bool) -> ParseArgs<'_> {
		ParseArgs {
			cursor: arg,
			quote_mode: false,
			escape_iter: None,
			is_arg0,
		}
	}
	/// Jump to the next argument. If there are any remaining characters in the
	/// current argument then they will be skipped.
	fn move_to_next_arg(&mut self) {
		while self.next().is_some() {}
		self.cursor.skip_whitespace();
		self.is_arg0 = false;
	}
}
impl Iterator for ParseArgs<'_> {
	type Item = u16;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			// Consume any possibly escaped characters.
			if let Some(iter) = self.escape_iter.as_mut() {
				match iter.next() {
					Some(w) => return Some(w),
					None => self.escape_iter = None,
				}
			}

			// Parse the arguments.
			match self.cursor.peek()? {
				SPACE | TAB if not(self.quote_mode) => {
					return None;
				}
				SLASH if not(self.is_arg0) => {
					self.cursor.next();
					let slashes = EscapeIter::new(&mut self.cursor);
					self.escape_iter = Some(slashes);
				}
				QUOTE => {
					self.cursor.next();
					if not(self.is_arg0) && self.quote_mode && self.cursor.peek() == Some(QUOTE) {
						return self.cursor.next();
					} else {
						self.quote_mode.toggle();
					}
				}
				_ => return self.cursor.next(),
			}
		}
	}
}

fn scalars<I: Iterator<Item = u16> + fmt::Debug + Clone>(
	iter: I,
) -> impl Iterator<Item = char> + fmt::Debug + Clone {
	decode_utf16(iter).map(|u| u.unwrap_or(REPLACEMENT_CHARACTER))
}
fn code_points<I: Iterator<Item = u16> + fmt::Debug + Clone>(
	iter: I,
) -> impl Iterator<Item = u32> + fmt::Debug + Clone {
	decode_utf16(iter).map(|u| match u {
		Ok(c) => c as u32,
		Err(e) => e.unpaired_surrogate() as u32,
	})
}

trait Toggle {
	fn toggle(&mut self);
}
impl Toggle for bool {
	#[inline(always)]
	fn toggle(&mut self) {
		*self = not(*self)
	}
}
#[inline(always)]
fn is_odd(v: u16) -> bool {
	v & 1 == 1
}
#[inline(always)]
fn not(v: bool) -> bool {
	!v
}

fn command_line() -> WideIter<'static> {
	// SAFETY: `GetCommandLineW` returns a 'static null terminated wide string.
	unsafe { WideIter::new(GetCommandLineW()) }
}

extern "system" {
	// GetCommandLineW cannot fail. The memory it points to must not be written
	// and must not be freed (i.e. it's 'static).
	fn GetCommandLineW() -> *const u16;
}
