//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//                    Version 2, December 2004
//
// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// Everyone is permitted to copy and distribute verbatim or modified
// copies of this license document, and changing it is allowed as long
// as the name is changed.
//
//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
//
//  0. You just DO WHAT THE FUCK YOU WANT TO.

use std::io;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
	Io(io::Error),
	NotFound,
	Parse,
	Expand(Expand),
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Expand {
	StackUnderflow,
	TypeMismatch,
	UnrecognizedFormatOption(u8),
	InvalidVariableName(u8),
	InvalidParameterIndex(u8),
	MalformedCharacterConstant,
	IntegerConstantOverflow,
	MalformedIntegerConstant,
	FormatWidthOverflow,
	FormatPrecisionOverflow,
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl From<io::Error> for Error {
	fn from(value: io::Error) -> Self {
		Error::Io(value)
	}
}

impl From<Expand> for Error {
	fn from(value: Expand) -> Self {
		Error::Expand(value)
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
		f.write_str(error::Error::description(self))
	}
}

impl error::Error for Error {
	fn description(&self) -> &str {
		match *self {
			Error::Io(ref err) =>
				err.description(),

			Error::NotFound =>
				"Capability database not found.",

			Error::Parse =>
				"Failed to parse capability database.",

			Error::Expand(ref err) =>
				match *err {
					Expand::StackUnderflow =>
						"Not enough elements on the stack.",

					Expand::TypeMismatch =>
						"Type mismatch.",

					Expand::UnrecognizedFormatOption(..) =>
						"Unrecognized format option.",

					Expand::InvalidVariableName(..) =>
						"Invalid variable name.",

					Expand::InvalidParameterIndex(..) =>
						"Invalid parameter index.",

					Expand::MalformedCharacterConstant =>
						"Malformed character constant.",

					Expand::IntegerConstantOverflow =>
						"Integer constant computation overflowed.",

					Expand::MalformedIntegerConstant =>
						"Malformed integer constant.",

					Expand::FormatWidthOverflow =>
						"Format width constant computation overflowed.",

					Expand::FormatPrecisionOverflow =>
						"Format precision constant computation overflowed.",
				},
		}
	}
}
