/*
 * Small example application that will dump the XML section of any E57 to stdout.
 */

use anyhow::{ensure, Context, Result};
use e57::E57Reader;
use std::fs::File;
use std::io::{stdout, BufReader, Write};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(args.len() >= 2, "Usage: e57-extract-xml <path/to/my.e57>");

    let file = File::open(&args[1]).context("Failed to open E57 file")?;
    let reader = BufReader::new(file);
    let xml = E57Reader::raw_xml(reader).context("Failed to extract XML data")?;

    stdout()
        .write_all(&xml)
        .context("Failed to write XML data to stdout")
}
