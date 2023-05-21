use anyhow::{Context, Result};
use std::process;

#[must_use]
pub fn do_something() -> Result<()> {
    Ok(())
}

pub fn main() {
    if let Err(err) = std::panic::catch_unwind(|| do_something()) {
        eprintln!("Error: {:?}", err);
        process::exit(1);
    }
}