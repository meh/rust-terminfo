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

macro_rules! all {
	($i:expr, $submac:ident!( $($args:tt)* )) => ({
		use ::nom::InputLength;

		let ret;
		let mut res = ::std::vec::Vec::new();
		let mut input = $i;

		loop {
			if input.input_len() == 0 {
				ret = ::nom::IResult::Done(input, res);
				break;
			}

			match $submac!(input, $($args)*) {
				::nom::IResult::Error(err) => {
					ret = ::nom::IResult::Error(err);
					break;
				}

				::nom::IResult::Incomplete(..) => {
					ret = ::nom::IResult::Incomplete(::nom::Needed::Unknown);
					break;
				}

				::nom::IResult::Done(i, o) => {
					if i == input {
						ret = ::nom::IResult::Error(error_position!(::nom::ErrorKind::Many0, input));
						break;
					}

					res.push(o);
					input = i;
				}
			}
		}

		ret
	});

	($i:expr, $f:expr) => (
		all!($i, call!($f));
	);
}
