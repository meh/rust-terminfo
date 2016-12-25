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

use error;

/// Trait for items that can be expanded.
pub trait Expand {
	fn expand(&self, parameters: &[Parameter], context: &mut Context) -> error::Result<Vec<u8>>;
}

/// An expansion parameter.
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Parameter {
	/// A number.
	Number(i32),

	/// An ASCII string.
	String(Vec<u8>),
}

impl Default for Parameter {
	fn default() -> Self {
		Parameter::Number(0)
	}
}

/// The expansion context.
///
/// The same context should be passed around through every expansion for the
/// same `Database`.
#[derive(Eq, PartialEq, Default, Debug)]
pub struct Context {
	pub fixed:   [Parameter; 26],
	pub dynamic: [Parameter; 26],
}

impl Expand for [u8] {
	fn expand(&self, parameters: &[Parameter], context: &mut Context) -> error::Result<Vec<u8>> {
		#[derive(Eq, PartialEq, Copy, Clone, Debug)]
		enum State {
			Input,
			Begin,
			Push,
			Variable(Variable),
			Constant(Constant),
			Format(Flags, FormatState),
			Seek(Seek),
		}

		#[derive(Eq, PartialEq, Copy, Clone, Debug)]
		enum Variable {
			Set,
			Get,
		}

		#[derive(Eq, PartialEq, Copy, Clone, Debug)]
		enum Constant {
			Character(bool),
			Integer(i32),
		}

		#[derive(Eq, PartialEq, Copy, Clone, Debug)]
		enum Seek {
			IfElse(usize),
			IfElseExpand(usize),
			IfEnd(usize),
			IfEndExpand(usize),
		}

		#[derive(Eq, PartialEq, Copy, Clone, Debug)]
		enum Format {
			Dec,
			Oct,
			Hex,
			HEX,
			Str,
		}

		#[derive(Eq, PartialEq, Copy, Clone, Debug)]
		enum FormatState {
			Flags,
			Width,
			Precision,
		}

		#[derive(Eq, PartialEq, Copy, Clone, Default, Debug)]
		struct Flags {
			width:     usize,
			precision: usize,

			alternate: bool,
			left:      bool,
			sign:      bool,
			space:     bool,
		}

		impl Format {
			pub fn from_char(c: u8) -> Format {
				match c {
					b'd' => Format::Dec,
					b'o' => Format::Oct,
					b'x' => Format::Hex,
					b'X' => Format::HEX,
					b's' => Format::Str,

					_ => unreachable!()
				}
			}

			pub fn format(&self, arg: &Parameter, flags: &Flags) -> error::Result<Vec<u8>> {
				use std::io::Write;
				use std::iter;

				let mut output = Vec::new();

				macro_rules! format {
					($($rest:tt)*) => (
						write!(output.by_ref(), $($rest)*)?
					);
				}

				match *arg {
					Parameter::Number(value) => {
						match *self {
							Format::Dec if flags.sign =>
								format!("{:+01$}", value, flags.precision),

							Format::Dec if value < 0 =>
								format!("{:01$}", value, flags.precision + 1),

							Format::Dec if flags.space =>
								format!(" {:01$}", value, flags.precision),

							Format::Dec =>
								format!("{:01$}", value, flags.precision),

							Format::Oct if flags.alternate =>
								format!("0{:01$o}", value, flags.precision.saturating_sub(1)),

							Format::Oct =>
								format!("{:01$o}", value, flags.precision),

							Format::Hex if flags.alternate && value != 0 =>
								format!("0x{:01$x}", value, flags.precision),

							Format::Hex =>
								format!("{:01$x}", value, flags.precision),

							Format::HEX if flags.alternate && value != 0 =>
								format!("0X{:01$X}", value, flags.precision),

							Format::HEX =>
								format!("{:01$X}", value, flags.precision),

							Format::Str =>
								return Err(error::Expand::TypeMismatch.into()),
						}
					}

					Parameter::String(ref value) => {
						match *self {
							Format::Str if flags.precision > 0 && flags.precision < value.len() =>
								output.extend(&value[..flags.precision]),

							Format::Str =>
								output.extend(value),

							_ =>
								return Err(error::Expand::TypeMismatch.into())
						}
					}
				}

				if flags.width > output.len() {
					let padding = flags.width - output.len();

					if flags.left {
						output.extend(iter::repeat(b' ').take(padding));
					}
					else {
						let mut padded = Vec::with_capacity(flags.width);
						padded.extend(iter::repeat(b' ').take(padding));
						padded.extend(output);

						output = padded;
					}
				}

				Ok(output)
			}
		}

		let mut output = Vec::with_capacity(self.len());
		let mut state  = State::Input;

		let mut stack                  = Vec::new();
		let mut params: [Parameter; 9] = Default::default();

		for (dest, source) in params.iter_mut().zip(parameters.iter()) {
			*dest = source.clone();
		}

		for ch in self.iter().cloned() {
			let mut old = state;

			match state {
				State::Input => {
					if ch == b'%' {
						state = State::Begin;
					}
					else {
						output.push(ch);
					}
				}

				State::Begin => {
					match ch {
						b'?' | b';' => (),

						b'%' => {
							output.push(b'%');
							state = State::Input;
						}

						b'c' => {
							match stack.pop() {
								Some(Parameter::Number(0)) =>
									output.push(128),

								Some(Parameter::Number(c)) =>
									output.push(c as u8),

								Some(Parameter::String(..)) =>
									return Err(error::Expand::TypeMismatch.into()),

								None =>
									return Err(error::Expand::StackUnderflow.into()),
							}
						}

						b'p' => {
							state = State::Push;
						}

						b'P' => {
							state = State::Variable(Variable::Set);
						}

						b'g' => {
							state = State::Variable(Variable::Get);
						}

						b'\\' => {
							state = State::Constant(Constant::Character(true));
						}

						b'{' => {
							state = State::Constant(Constant::Integer(0));
						}

						b'+' | b'-' | b'/' | b'*' | b'^' | b'&' | b'|' | b'm' => {
							match (stack.pop(), stack.pop()) {
								(Some(Parameter::Number(y)), Some(Parameter::Number(x))) =>
									stack.push(Parameter::Number(match ch {
										b'+' => x + y,
										b'-' => x - y,
										b'/' => x / y,
										b'*' => x * y,
										b'^' => x ^ y,
										b'&' => x & y,
										b'|' => x | y,
										b'm' => x % y,

										_ => unreachable!()
									})),

								(Some(_), Some(_)) =>
									return Err(error::Expand::TypeMismatch.into()),

								_ =>
									return Err(error::Expand::StackUnderflow.into()),
							}
						}

						b'=' | b'>' | b'<' | b'A' | b'O' => {
							match (stack.pop(), stack.pop()) {
								(Some(Parameter::Number(y)), Some(Parameter::Number(x))) =>
									stack.push(Parameter::Number(match ch {
										b'=' => x == y,
										b'<' => x < y,
										b'>' => x > y,
										b'A' => x > 0 && y > 0,
										b'O' => x > 0 || y > 0,

										_ => unreachable!()
									} as i32)),

								(Some(_), Some(_)) =>
									return Err(error::Expand::TypeMismatch.into()),

								_ =>
									return Err(error::Expand::StackUnderflow.into()),
							}
						}

						b'!' | b'~' => {
							match stack.pop() {
								Some(Parameter::Number(x)) =>
									stack.push(Parameter::Number(match ch {
										b'!' if x > 0 => 0,
										b'!'          => 1,
										b'~'          => !x,

										_ => unreachable!()
									})),

								Some(_) =>
									return Err(error::Expand::TypeMismatch.into()),

								_ =>
									return Err(error::Expand::StackUnderflow.into()),
							}
						}

						b'i' => {
							if let (&Parameter::Number(x), &Parameter::Number(y)) = (&params[0], &params[1]) {
								params[0] = Parameter::Number(x + 1);
								params[1] = Parameter::Number(y + 1);
							}
							else {
								return Err(error::Expand::TypeMismatch.into());
							}
						}

						b'd' | b'o' | b'x' | b'X' | b's' => {
							if let Some(arg) = stack.pop() {
								output.extend(Format::from_char(ch).format(&arg, &Default::default())?);
							}
							else {
								return Err(error::Expand::StackUnderflow.into());
							}
						}

						b':' | b'#' | b' ' | b'.' | b'0' ... b'9' => {
							let mut flags = Flags::default();
							let mut inner = FormatState::Flags;

							match ch {
								b':' => (),

								b'#' =>
									flags.alternate = true,

								b' ' =>
									flags.space = true,

								b'.' =>
									inner = FormatState::Precision,

								b'0' ... b'9' => {
									flags.width = ch as usize - '0' as usize;
									inner = FormatState::Width;
								}

								_ => unreachable!()
							}

							state = State::Format(flags, inner);
						}

						b't' => {
							match stack.pop() {
								Some(Parameter::Number(0)) =>
									state = State::Seek(Seek::IfElse(0)),

								Some(Parameter::Number(_)) =>
									(),

								Some(_) =>
									return Err(error::Expand::TypeMismatch.into()),

								None =>
									return Err(error::Expand::StackUnderflow.into()),
							}
						}

						b'e' => {
							state = State::Seek(Seek::IfEnd(0));
						}

						c => {
							return Err(error::Expand::UnrecognizedFormatOption(c).into());
						}
					}
				}

				State::Push => {
					if let Some(d) = (ch as char).to_digit(10) {
						stack.push(params[d as usize - 1].clone());
					}
					else {
						return Err(error::Expand::InvalidParameterIndex(ch).into());
					}
				}
	
				State::Variable(Variable::Set) => {
					match ch {
						b'A' ... b'Z' => {
							if let Some(arg) = stack.pop() {
								context.fixed[(ch - b'A') as usize] = arg;
							}
							else {
								return Err(error::Expand::StackUnderflow.into());
							}
						}
	
						b'a' ... b'z' => {
							if let Some(arg) = stack.pop() {
								context.dynamic[(ch - b'a') as usize] = arg;
							}
							else {
								return Err(error::Expand::StackUnderflow.into());
							}
						}
	
						_ =>
							return Err(error::Expand::InvalidVariableName(ch).into())
					}
				}

				State::Variable(Variable::Get) => {
					match ch {
						b'A' ... b'Z' => {
							stack.push(context.fixed[(ch - b'A') as usize].clone());
						}

						b'a' ... b'z' => {
							stack.push(context.fixed[(ch - b'a') as usize].clone());
						}

						_ =>
							return Err(error::Expand::InvalidVariableName(ch).into())
					}
				}

				State::Constant(Constant::Character(true)) => {
					stack.push(Parameter::Number(ch as i32));
					state = State::Constant(Constant::Character(false));
				}

				State::Constant(Constant::Character(false)) => {
					if ch != b'\'' {
						return Err(error::Expand::MalformedCharacterConstant.into());
					}
				}

				State::Constant(Constant::Integer(number)) => {
					if ch == b'}' {
						stack.push(Parameter::Number(number));
					}
					else if let Some(digit) = (ch as char).to_digit(10) {
						if let Some(number) = number.checked_mul(10).and_then(|n| n.checked_add(digit as i32)) {
							state = State::Constant(Constant::Integer(number));
							old   = State::Input;
						}
						else {
							return Err(error::Expand::IntegerConstantOverflow.into());
						}
					}
					else {
						return Err(error::Expand::MalformedIntegerConstant.into());
					}
				}

				State::Format(ref mut flags, ref mut inner) => {
					old = State::Input;

					match (*inner, ch) {
						(_, b'd') | (_, b'o') | (_, b'x') | (_, b'X') | (_, b's') => {
							if let Some(arg) = stack.pop() {
								output.extend(Format::from_char(ch).format(&arg, flags)?);
								old = State::Format(*flags, *inner);
							}
							else {
								return Err(error::Expand::StackUnderflow.into());
							}
						}

						(FormatState::Flags, b'#') => {
							flags.alternate = true;
						}

						(FormatState::Flags, b'-') => {
							flags.left = true;
						}

						(FormatState::Flags, b'+') => {
							flags.sign = true;
						}

						(FormatState::Flags, b' ') => {
							flags.space = true;
						}

						(FormatState::Flags, b'0' ... b'9') => {
							flags.width = ch as usize - b'0' as usize;
							*inner = FormatState::Width;
						}

						(FormatState::Width, b'0' ... b'9') => {
							if let Some(width) = flags.width.checked_mul(10).and_then(|w| w.checked_add(ch as usize - b'0' as usize)) {
								flags.width = width;
							}
							else {
								return Err(error::Expand::FormatWidthOverflow.into());
							}
						}

						(FormatState::Width, b'.') | (FormatState::Flags, b'.') => {
							*inner = FormatState::Precision;
						}

						(FormatState::Precision, b'0' ... b'9') => {
							if let Some(precision) = flags.precision.checked_mul(10).and_then(|p| p.checked_add(ch as usize - b'0' as usize)) {
								flags.precision = precision;
							}
							else {
								return Err(error::Expand::UnrecognizedFormatOption(ch).into());
							}
						}

						_ =>
							return Err(error::Expand::UnrecognizedFormatOption(ch).into())
					}
				}

				State::Seek(Seek::IfElse(level)) => {
					if ch == b'%' {
						state = State::Seek(Seek::IfElseExpand(level));
					}

					old = State::Input;
				}

				State::Seek(Seek::IfElseExpand(level)) => {
					state = match ch {
						b';' if level == 0 =>
							State::Input,

						b';' =>
							State::Seek(Seek::IfElse(level - 1)),

						b'e' if level == 0 =>
							State::Input,

						b'?' =>
							State::Seek(Seek::IfElse(level + 1)),

						_ =>
							State::Seek(Seek::IfElse(level))
					};
				}

				State::Seek(Seek::IfEnd(level)) => {
					if ch == b'%' {
						state = State::Seek(Seek::IfEndExpand(level));
					}

					old = State::Input;
				}

				State::Seek(Seek::IfEndExpand(level)) => {
					state = match ch {
						b';' if level == 0 =>
							State::Input,

						b';' =>
							State::Seek(Seek::IfEnd(level - 1)),

						b'?' =>
							State::Seek(Seek::IfEnd(level + 1)),

						_ =>
							State::Seek(Seek::IfEnd(level))
					};
				}
			}

			if state == old {
				state = State::Input;
			}
		}

		Ok(output)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_basic_setabf() {
		assert_eq!(b"\\E[48;5;1m".to_vec(),
			Expand::expand(&b"\\E[48;5;%p1%dm"[..], &[Parameter::Number(1)], &mut Default::default()).unwrap());
	}
}
