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

use fnv::FnvHasher;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::hash::BuildHasherDefault;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::capability::{Capability, Value};
use crate::error::{self, Error};
use crate::names;
use crate::parser::compiled;

/// A capability database.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Database {
	/// The terminal name.
	pub name: String,

	/// Aliases of the terminal.
	pub aliases: Vec<String>,

	/// The terminal description.
	pub description: String,

	inner: HashMap<String, Value, BuildHasherDefault<FnvHasher>>,
}

impl Database {
	/// Create a new empty terminfo database with its name and description.
	pub fn new<N: Into<String>, D: Into<String>>(name: N, description: D) -> Self {
		Self {
			name: name.into(),
			aliases: Vec::new(),
			description: description.into(),
			inner: HashMap::default(),
		}
	}

	/// Load a database from the current environment.
	pub fn from_env() -> error::Result<Self> {
		if let Ok(name) = env::var("TERM") {
			Self::from_name(name)
		} else {
			Err(Error::NotFound)
		}
	}

	/// Load a database for the given name.
	pub fn from_name<N: AsRef<str>>(name: N) -> error::Result<Self> {
		let name = name.as_ref();
		let first = name.chars().next().ok_or(Error::NotFound)?;

		// See https://manpages.debian.org/buster/ncurses-bin/terminfo.5.en.html#Fetching_Compiled_Descriptions
		let mut search = Vec::<PathBuf>::new();

		if let Some(dir) = env::var_os("TERMINFO") {
			search.push(dir.into());
		} else if let Some(mut home) = dirs::home_dir() {
			home.push(".terminfo");
			search.push(home);
		}

		if let Ok(dirs) = env::var("TERMINFO_DIRS") {
			for dir in dirs.split(':') {
				search.push(dir.into());
			}
		}

		// handle non-FHS systems like Termux
		if let Ok(prefix) = env::var("PREFIX") {
			let path = Path::new(&prefix);
			search.push(path.join("etc/terminfo"));
			search.push(path.join("lib/terminfo"));
			search.push(path.join("share/terminfo"));
		}

		search.push("/etc/terminfo".into());
		search.push("/lib/terminfo".into());
		search.push("/usr/share/terminfo".into());
		search.push("/boot/system/data/terminfo".into());

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
		if let Ok((_, database)) = compiled::parse(buffer.as_ref()) {
			Ok(database.into())
		} else {
			Err(Error::Parse)
		}
	}

	/// Get a capability.
	///
	/// ## Example
	///
	/// ```
	/// use terminfo::{Database, capability as cap};
	///
	/// let info        = Database::from_env().unwrap();
	/// let colors: i32 = info.get::<cap::MaxColors>().unwrap().into();
	/// ```
	pub fn get<'a, C: Capability<'a>>(&'a self) -> Option<C> {
		C::from(self.inner.get(C::name()))
	}

	/// Set a capability.
	///
	/// ## Example
	///
	/// ```
	/// let mut info = terminfo::Database::new("foo", "foo terminal");
	///
	/// // Set the amount of available colors.
	/// info.set(terminfo::capability::MaxColors(16));
	/// ```
	pub fn set<'a, C: Capability<'a>>(&mut self, value: C) {
		if !self.inner.contains_key(C::name()) {
			if let Some(value) = C::into(value) {
				self.inner.insert(C::name().into(), value);
			}
		}
	}

	/// Get a capability by name.
	///
	/// ## Note
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
		let name = names::ALIASES.get(name).copied().unwrap_or(name);

		self.inner.get(name)
	}

	/// Set a raw capability.
	///
	/// ## Note
	///
	/// This interface only makes sense for extended capabilities since they
	/// don't have standardized types.
	///
	/// ## Example
	///
	/// ```
	/// let mut info = terminfo::Database::new("foo", "foo terminal");
	///
	/// // Set the amount of available colors.
	/// info.set_raw("colors", 16);
	/// ```
	pub fn set_raw<S: AsRef<str>, V: Into<Value>>(&mut self, name: S, value: V) {
		let name = name.as_ref();
		let name = names::ALIASES.get(name).copied().unwrap_or(name);

		if !self.inner.contains_key(name) {
			self.inner.insert(name.into(), value.into());
		}
	}
}
