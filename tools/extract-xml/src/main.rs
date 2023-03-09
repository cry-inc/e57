/*
 * Small example application that will dump the XML section of any E57 to stdout.
 */

use anyhow::{bail, Context, Result};
use e57::E57;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        bail!("Usage: extract-xml <path/to/my.e57>");
    }

    let file = E57::from_file(&args[1]).context("Failed to open E57 file")?;
    let xml = file.raw_xml();
    println!("{xml}");

    Ok(())
}
