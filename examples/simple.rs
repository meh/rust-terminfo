#[macro_use(expand)]
extern crate terminfo;

use std::io;
use terminfo::{Database, capability as cap};

fn main() {
  let info = Database::from_env().unwrap();

  if let Some(cap::MaxColors(n)) = info.get::<cap::MaxColors>() {
    println!("The terminal supports {} colors.", n);
  }
  else {
    println!("The terminal does not support colors, what year is this?");
  }

  if let Some(flash) = info.get::<cap::FlashScreen>() {
		expand!(io::stdout(), flash).unwrap();
  }
	else {
		println!("FLASH GORDON!");
	}

	expand!(io::stdout(), info.get::<cap::SetAForeground>().unwrap(); 1).unwrap();
	expand!(io::stdout(), info.get::<cap::SetABackground>().unwrap(); 4).unwrap();
	println!("SUP");
	expand!(io::stdout(), info.get::<cap::ExitAttributeMode>().unwrap()).unwrap();
}
