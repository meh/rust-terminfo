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
//! A capability database.

use fnv::FnvHasher;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::hash::BuildHasherDefault;
use std::path::{Path, PathBuf};
use std::{env, io};

use crate::capability::{Capability, Value};
use crate::names;
use crate::parser::compiled;

/// A capability database.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Database {
	name: String,
	aliases: Vec<String>,
	description: String,
	inner: HashMap<String, Value, BuildHasherDefault<FnvHasher>>,
}

/// Builder for a new `Database`.
#[derive(Default, Debug)]
pub struct Builder {
	name: Option<String>,
	aliases: Vec<String>,
	description: Option<String>,
	inner: HashMap<String, Value, BuildHasherDefault<FnvHasher>>,
}

impl Builder {
	/// Build the database.
	pub fn build(self) -> Result<Database, ()> {
		Ok(Database {
			name: self.name.ok_or(())?,
			aliases: self.aliases,
			description: self.description.unwrap_or_default(),
			inner: self.inner,
		})
	}

	/// Set the terminal name.
	pub fn name<T: Into<String>>(&mut self, name: T) -> &mut Self {
		self.name = Some(name.into());
		self
	}

	/// Set the terminal aliases.
	pub fn aliases<T, I>(&mut self, iter: I) -> &mut Self
	where
		T: Into<String>,
		I: IntoIterator<Item = T>,
	{
		self.aliases = iter.into_iter().map(|a| a.into()).collect();
		self
	}

	/// Set the terminal description.
	pub fn description<T: Into<String>>(&mut self, description: T) -> &mut Self {
		self.description = Some(description.into());
		self
	}

	/// Set a capability.
	///
	/// ## Example
	///
	/// ```
	/// use terminfo::{Database, capability as cap};
	///
	/// let mut info = Database::new();
	/// info.name("foo");
	/// info.description("foo terminal");
	///
	/// // Set the amount of available colors.
	/// info.set(cap::MaxColors(16));
	///
	/// info.build().unwrap();
	/// ```
	pub fn set<'a, C: Capability<'a>>(&'a mut self, value: C) -> &'a mut Self {
		if !self.inner.contains_key(C::name()) {
			if let Some(value) = C::into(value) {
				self.inner.insert(C::name().into(), value);
			}
		}

		self
	}

	/// Set a raw capability.
	///
	/// ## Example
	///
	/// ```
	/// use terminfo::{Database, capability as cap};
	///
	/// let mut info = Database::new();
	/// info.name("foo");
	/// info.description("foo terminal");
	///
	/// // Set the amount of available colors.
	/// info.raw("colors", 16);
	///
	/// info.build().unwrap();
	/// ```
	pub fn raw<S: AsRef<str>, V: Into<Value>>(&mut self, name: S, value: V) -> &mut Self {
		let name = name.as_ref();
		let name = names::ALIASES.get(name).copied().unwrap_or(name);

		if !self.inner.contains_key(name) {
			self.inner.insert(name.into(), value.into());
		}

		self
	}
}

impl Database {
	/// Create a database builder for constucting a database.
	// Clippy is right, the naming is is unconventional, but it’s probably not worth changing
	#[allow(clippy::new_ret_no_self)]
	pub fn new() -> Builder {
		Builder::default()
	}

	/// Load a database from the current environment.
	pub fn from_env() -> Result<Self, FromEnvError> {
		if let Ok(name) = env::var("TERM") {
			Self::from_name(name).map_err(FromEnvError::FromName)
		} else {
			Err(FromEnvError::NoTerm(NoTerm))
		}
	}

	/// Load a database for the given name.
	pub fn from_name<N: AsRef<str>>(name: N) -> Result<Self, FromNameError> {
		let name = name.as_ref();
		let not_found = || FromNameError::NotFound(NotFound { name: name.into() });
		let first = name.chars().next().ok_or_else(not_found)?;

		// See https://manpages.debian.org/buster/ncurses-bin/terminfo.5.en.html#Fetching_Compiled_Descriptions
		let mut search = Vec::<PathBuf>::new();

		#[allow(deprecated)]
		if let Some(dir) = env::var_os("TERMINFO") {
			search.push(dir.into());
		} else if let Some(mut home) = std::env::home_dir() {
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
		search.push("/usr/local/share/terminfo".into());
		search.push("/usr/local/share/site-terminfo".into());
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
					return Self::from_path(path).map_err(FromNameError::Load);
				}
			}

			// Check non-standard location.
			{
				let mut path = path.clone();
				path.push(format!("{:x}", first as usize));
				path.push(name);

				if fs::metadata(&path).is_ok() {
					return Self::from_path(path).map_err(FromNameError::Load);
				}
			}
		}

		Err(not_found())
	}

	/// Load a database from the given path.
	pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
		let path = path.as_ref();
		(|| {
			let buffer = fs::read(path).map_err(LoadErrorKind::Read)?;
			Self::from_buffer(buffer).map_err(LoadErrorKind::Parse)
		})()
		.map_err(|kind| LoadError { path: path.into(), kind })
	}

	/// Load a database from a buffer.
	pub fn from_buffer<T: AsRef<[u8]>>(buffer: T) -> Result<Self, ParseError> {
		if let Ok((_, database)) = compiled::parse(buffer.as_ref()) {
			Ok(database.into())
		} else {
			Err(ParseError)
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
	/// let colors: i32 = info.get::<cap::MaxColors>().unwrap().into();
	/// ```
	pub fn get<'a, C: Capability<'a>>(&'a self) -> Option<C> {
		C::from(self.inner.get(C::name()))
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
}

/// An error in [`Database::from_env`].
#[derive(Debug)]
pub enum FromEnvError {
	/// The `$TERM` environment variable was not set.
	NoTerm(NoTerm),
	/// The terminal name was read, but loading the database failed.
	FromName(FromNameError),
}

impl Display for FromEnvError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str("failed to load terminfo database")
	}
}

impl Error for FromEnvError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::NoTerm(e) => Some(e),
			Self::FromName(e) => e.source(),
		}
	}
}

/// The `$TERM` environment variable was not set.
///
/// A root cause of [`FromEnvError`].
#[derive(Debug)]
#[non_exhaustive]
pub struct NoTerm;

impl Display for NoTerm {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str("no `$TERM` environment variable")
	}
}

impl Error for NoTerm {}

/// An error in [`Database::from_name`].
#[derive(Debug)]
pub enum FromNameError {
	/// The terminfo entry was not found.
	NotFound(NotFound),
	/// The terminfo file could not be loaded.
	Load(LoadError),
}

impl Display for FromNameError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str("failed to load terminfo database")
	}
}

impl Error for FromNameError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::NotFound(e) => Some(e),
			Self::Load(e) => Some(e),
		}
	}
}

/// No terminfo database was found.
///
/// A root cause of [`FromNameError`].
#[derive(Debug)]
#[non_exhaustive]
pub struct NotFound {
	name: Box<str>,
}

impl NotFound {
	/// Get the name of the terminfo database.
	#[must_use]
	pub fn name(&self) -> &str {
		&self.name
	}
}

impl Display for NotFound {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "no terminfo database found for `{}`", self.name)
	}
}

impl Error for NotFound {}

/// An error loading the database, returned by [`Database::from_path`].
#[derive(Debug)]
#[non_exhaustive]
pub struct LoadError {
	/// The path the database is located at.
	pub path: Box<Path>,
	/// The cause of the error.
	pub kind: LoadErrorKind,
}

impl Display for LoadError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "failed to load terminfo database {}", self.path.display())
	}
}

impl Error for LoadError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match &self.kind {
			LoadErrorKind::Read(e) => Some(e),
			LoadErrorKind::Parse(e) => Some(e),
		}
	}
}

/// A cause of a [`LoadError`].
#[derive(Debug)]
pub enum LoadErrorKind {
	/// An error occurred reading the file.
	Read(io::Error),
	/// There was an error parsing the file.
	Parse(ParseError),
}

/// An error parsing the database, returned by [`Database::from_buffer`].
#[derive(Debug)]
#[non_exhaustive]
pub struct ParseError;

impl Display for ParseError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str("failed to parse terminfo database")
	}
}

impl Error for ParseError {}
