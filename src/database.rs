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

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Read;

use capability::{Capability, Value};
use names::*;
use error::{self, Error};
use nom::IResult;
use parser::compiled;

#[derive(Eq, PartialEq, Clone, Debug, Default)]
pub struct Database {
	name:        String,
	aliases:     Vec<String>,
	description: String,

	inner: HashMap<String, Value>,
}

impl Database {
	pub fn new(name: String, aliases: Vec<String>, description: String, inner: HashMap<String, Value>) -> Self {
		Database {
			name:        name,
			aliases:     aliases,
			description: description,

			inner: inner,
		}
	}

	pub fn from_env() -> error::Result<Self> {
		if let Ok(name) = env::var("TERM") {
			Self::from_name(name)
		}
		else {
			Err(Error::NotFound)
		}
	}

	pub fn from_name<N: AsRef<str>>(name: N) -> error::Result<Self> {
		let name  = name.as_ref();
		let first = name.chars().next().ok_or(Error::NotFound)?;

		let mut search = Vec::<PathBuf>::new();

		if let Some(dir) = env::var_os("TERMINFO") {
			search.push(dir.into());
		}

		if let Ok(dirs) = env::var("TERMINFO_DIRS") {
			for dir in dirs.split(':') {
				if dir.is_empty() {
					search.push("/usr/share/terminfo".into());
				}
				else {
					search.push(dir.into());
				}
			}
		}
		else {
			if let Some(mut home) = env::home_dir() {
				home.push(".terminfo");
				search.push(home.into());
			}

			search.push("/etc/terminfo".into());
			search.push("/lib/terminfo".into());
			search.push("/usr/share/terminfo".into());
			search.push("/boot/system/data/terminfo".into());
		}

		for path in search {
			if fs::metadata(&path).is_err() {
				continue;
			}

			// Check standard location.
			{
				let mut path = path.clone();
				path.push(first.to_string());
				path.push(name);

				if fs::metadata(&path).is_ok() {
					return Self::from_path(path);
				}
			}

			// Check non-standard location.
			{
				let mut path = path.clone();
				path.push(format!("{:x}", first as usize));
				path.push(name);

				if fs::metadata(&path).is_ok() {
					return Self::from_path(path);
				}
			}
		}

		Err(Error::NotFound)
	}

	pub fn from_path<P: AsRef<Path>>(path: P) -> error::Result<Self> {
		let mut file = File::open(path)?;
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer)?;

		if let IResult::Done(_, database) = compiled::parse(&buffer) {
			Ok(database.into())
		}
		else {
			Err(Error::Parse)
		}
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn aliases(&self) -> &[String] {
		&self.aliases
	}

	pub fn description(&self) -> &str {
		&self.description
	}

	pub fn get<'a, C: Capability<'a>>(&'a self) -> Option<C> {
		C::parse(self.inner.get(C::name()))
	}

	pub fn raw<S: AsRef<str>>(&self, name: S) -> Option<&Value> {
		let name = name.as_ref();

		self.inner.get(if let Some(index) = BOOLEAN_NAMES.iter().position(|&n| n == name) {
			BOOLEAN_LONG_NAMES[index]
		}
		else if let Some(index) = NUMBER_NAMES.iter().position(|&n| n == name) {
			NUMBER_LONG_NAMES[index]
		}
		else if let Some(index) = STRING_NAMES.iter().position(|&n| n == name) {
			STRING_LONG_NAMES[index]
		}
		else {
			name
		})
	}
}
