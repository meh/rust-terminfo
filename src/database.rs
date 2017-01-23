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
use names;
use error::{self, Error};
use nom::IResult;
use parser::compiled;

/// A capability database.
#[derive(Eq, PartialEq, Clone, Debug, Default)]
pub struct Database {
	name:        String,
	aliases:     Vec<String>,
	description: String,

	inner: HashMap<String, Value>,
}

impl Database {
	/// Create a new database from the given values.
	pub fn new(name: String, aliases: Vec<String>, description: String, inner: HashMap<String, Value>) -> Self {
		Database {
			name:        name,
			aliases:     aliases,
			description: description,

			inner: inner,
		}
	}

	/// Load a database from the current environment.
	pub fn from_env() -> error::Result<Self> {
		if let Ok(name) = env::var("TERM") {
			Self::from_name(name)
		}
		else {
			Err(Error::NotFound)
		}
	}

	/// Load a database for the given name.
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

	/// Load a database from the given path.
	pub fn from_path<P: AsRef<Path>>(path: P) -> error::Result<Self> {
		let mut file = File::open(path)?;
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer)?;

		Self::from_buffer(buffer)
	}

	/// Load a database from a buffer.
	pub fn from_buffer<T: AsRef<[u8]>>(buffer: T) -> error::Result<Self> {
		if let IResult::Done(_, database) = compiled::parse(buffer.as_ref()) {
			Ok(database.into())
		}
		else {
			Err(Error::Parse)
		}
	}

	/// The terminal name.
	pub fn name(&self) -> &str {
		&self.name
	}

	/// The terminal aliases.
	pub fn aliases(&self) -> &[String] {
		&self.aliases
	}

	/// The terminal description.
	pub fn description(&self) -> &str {
		&self.description
	}

	/// Get a capability.
	///
	/// ## Example
	///
	/// ```
	/// use terminfo::{Database, capability as cap};
	///
	/// let info        = Database::from_env().unwrap();
	/// let colors: i16 = info.get::<cap::MaxColors>().unwrap().into();
	/// ```
	pub fn get<'a, C: Capability<'a>>(&'a self) -> Option<C> {
		C::lookup(self)
	}

	/// Get a capability by name.
	///
	/// This interface only makes sense for extended capabilities since they
	/// don't have standardized types.
	///
	/// ## Example
	///
	/// ```
	/// use terminfo::Database;
	///
	/// let info      = Database::from_env().unwrap();
	/// let truecolor = info.raw("Tc").is_some();
	/// ```
	pub fn raw<S: AsRef<str>>(&self, name: S) -> Option<&Value> {
		let name = name.as_ref();

		self.inner.get(name).or_else(||
			names::ALIASES.get(name).and_then(|&name| self.inner.get(name)))
	}
}
