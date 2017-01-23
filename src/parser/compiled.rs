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

use std::str;
use std::collections::HashMap;

use nom::le_i16;
use names;
use capability::Value;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Database<'a> {
	names:    &'a [u8],
	standard: Standard<'a>,
	extended: Option<Extended<'a>>,
}

impl<'a> Into<::Database> for Database<'a> {
	fn into(self) -> ::Database {
		let mut names = self.names.split(|&c| c == b'|')
			.map(|s| unsafe { str::from_utf8_unchecked(s) })
			.map(|s| s.trim())
			.collect::<Vec<_>>();

		let name        = names.remove(0).to_string();
		let description = names.pop().unwrap().to_string();
		let aliases     = names.iter().map(|s| s.to_string()).collect();

		let mut capabilities = HashMap::new();

		for (index, _) in self.standard.booleans.iter().enumerate().filter(|&(_, &value)| value) {
			if let Some(&name) = names::BOOLEAN.get(&(index as u16)) {
				capabilities.entry(name.into())
					.or_insert(Value::True);
			}
		}

		for (index, &value) in self.standard.numbers.iter().enumerate().filter(|&(_, &n)| n >= 0) {
			if let Some(&name) = names::NUMBER.get(&(index as u16)) {
				capabilities.entry(name.into())
					.or_insert(Value::Number(value));
			}
		}

		for (index, &offset) in self.standard.strings.iter().enumerate().filter(|&(_, &n)| n >= 0) {
			if let Some(&name) = names::STRING.get(&(index as u16)) {
				let string = &self.standard.table[offset as usize ..];
				let edge   = string.iter().position(|&c| c == 0).unwrap();

				capabilities.entry(name.into())
					.or_insert(Value::String(Vec::from(&string[.. edge])));
			}
		}

		if let Some(extended) = self.extended {
			let names = extended.table.split(|&c| c == 0)
				.skip(extended.strings.iter().cloned().filter(|&n| n >= 0).count())
				.map(|s| unsafe { str::from_utf8_unchecked(s) })
				.collect::<Vec<_>>();

			for (index, _) in extended.booleans.iter().enumerate().filter(|&(_, &value)| value)  {
				capabilities.entry(names[index].into())
					.or_insert(Value::True);
			}

			for (index, &value) in extended.numbers.iter().enumerate().filter(|&(_, &n)| n >= 0) {
				capabilities.entry(names[extended.booleans.len() + index].into())
					.or_insert(Value::Number(value));
			}

			for (index, &offset) in extended.strings.iter().enumerate().filter(|&(_, &n)| n >= 0) {
				let string = &extended.table[offset as usize ..];
				let edge   = string.iter().position(|&c| c == 0).unwrap();

				capabilities.entry(names[extended.booleans.len() + extended.numbers.len() + index].into())
					.or_insert(Value::String(Vec::from(&string[.. edge])));
			}
		}

		::Database::new(name, aliases, description, capabilities)
	}
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Standard<'a> {
	booleans: Vec<bool>,
	numbers:  Vec<i16>,
	strings:  Vec<i16>,
	table:    &'a [u8],
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Extended<'a> {
	booleans: Vec<bool>,
	numbers:  Vec<i16>,
	strings:  Vec<i16>,
	names:    Vec<i16>,
	table:    &'a [u8],
}

named!(pub parse<Database>,
	do_parse!(
		tag!([0x1A, 0x01]) >>

		name_size:    size >>
		bool_count:   size >>
		num_count:    size >>
		string_count: size >>
		table_size:   size >>

		names: flat_map!(take!(name_size),
			take_until!("\x00")) >>

		booleans: flat_map!(take!(bool_count),
			all!(boolean)) >>

		cond!((name_size + bool_count) % 2 != 0,
			take!(1)) >>

		numbers: flat_map!(take!(num_count * 2),
			all!(capability)) >>

		strings: flat_map!(take!(string_count * 2),
			all!(capability)) >>

		table: take!(table_size) >>

		extended: opt!(complete!(do_parse!(
			cond!(table_size % 2 != 0,
				take!(1)) >>

			ext_bool_count:   size >>
			ext_num_count:    size >>
			ext_string_count: size >>
			ext_offset_count: size >>
			ext_table_size:   size >>

			booleans: flat_map!(take!(ext_bool_count),
				all!(boolean)) >>

			cond!(ext_bool_count % 2 != 0,
				take!(1)) >>

			numbers: flat_map!(take!(ext_num_count * 2),
				all!(capability)) >>

			strings: flat_map!(take!(ext_string_count * 2),
				all!(capability)) >>

			names: flat_map!(take!((ext_bool_count + ext_num_count + ext_string_count) * 2),
				all!(capability)) >>

			table: take!(ext_table_size) >>

			(Extended {
				booleans: booleans,
				numbers:  numbers,
				strings:  strings,
				names:    names,
				table:    table,
			})))) >>

		(Database {
			names: names,

			standard: Standard {
				booleans: booleans,
				numbers:  numbers,
				strings:  strings,
				table:    table,
			},

			extended: extended,
		})));

named!(boolean<bool>,
	alt!(tag!([0]) => { |_| false } |
	     tag!([1]) => { |_| true }));

named!(size<i16>,
	map_opt!(le_i16, |n| match n {
		-1          => Some(0),
		n if n >= 0 => Some(n),
		_           => None }));

named!(capability<i16>,
	map_opt!(le_i16, |n| if n >= -2 { Some(n) } else { None }));

#[cfg(test)]
mod test {
	use std::fs::File;
	use std::io::Read;
	use std::path::Path;
	use super::*;
	use capability as cap;

	fn load<P: AsRef<Path>, F: FnOnce(::Database)>(path: P, f: F) {
		let mut file   = File::open(path).unwrap();
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer).unwrap();

		f(parse(&buffer).unwrap().1.into())
	}

	#[test]
	fn name() {
		load("tests/cancer-256color", |db|
			assert_eq!("cancer-256color", db.name()));
	}

	#[test]
	fn aliases() {
		load("tests/st-256color", |db|
			assert_eq!(vec!["stterm-256color"], db.aliases()));
	}

	#[test]
	fn description() {
		load("tests/cancer-256color", |db|
			assert_eq!("terminal cancer with 256 colors", db.description()));
	}

	#[test]
	fn standard() {
		load("tests/st-256color", |db| {
			assert_eq!(Some(cap::Columns(80)), db.get::<cap::Columns>());
			assert_eq!(Some(cap::AutoRightMargin(true)), db.get::<cap::AutoRightMargin>());
			assert_eq!(Some(cap::AutoLeftMargin(false)), db.get::<cap::AutoLeftMargin>());
		});
	}

	#[test]
	fn extended() {
		load("tests/cancer-256color", |db| {
			assert_eq!(Some(&cap::Value::True), db.raw("Ts"));
			assert_eq!(Some(&cap::Value::True), db.raw("AX"));
			assert_eq!(Some(&cap::Value::String(b"\x1B[2 q".to_vec())), db.raw("Se"));
		});
	}
}
