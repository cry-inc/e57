/*
 * Small example application that will dump the XML section of any E57 to stdout.
 */

use anyhow::{bail, Context, Result};
use e57::E57;
use std::fs::File;
use std::io::{stdout, Write};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        bail!("Usage: extract-xml <path/to/my.e57>");
    }

    let file = File::open(&args[1]).context("Failed to open E57 file")?;
    let xml = E57::raw_xml(file).context("Failed to extract XML data")?;

    stdout()
        .write_all(&xml)
        .context("Failed to write XML data to stdout")
}
