/*
 * Small example application that will validate all CRC checksums of E57 files.
 * If the argument is a file path, it will check a single file.
 * If the argument is a directory, will check recurisvely all E57 files in that directory.
 */

use anyhow::{anyhow, bail, Context, Result};
use e57::E57Reader;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        bail!("Usage:\n  crc-validator <path/to/my.e57>\n  crc-validator <path/to/folder/>");
    }

    let path_str = &args[1];
    let path = Path::new(path_str);
    if !path.exists() {
        bail!("The path '{path_str}' does not exist");
    }

    let all_ok = if path.is_dir() {
        let files = list_files(path).context("Failed to list E57 files")?;
        println!("Found {} files, starting validation...", files.len());
        check_files(&files)
    } else if path.is_file() {
        check_file(path_str)
    } else {
        bail!("The path '{path_str}' does not point to a directory or a file");
    };

    if all_ok {
        println!("All files are okay!");
        Ok(())
    } else {
        Err(anyhow!("Some of the checked files are not okay"))
    }
}

fn list_files(path: &Path) -> Result<Vec<String>> {
    let mut res = Vec::new();
    for entry in path.read_dir().expect("Failed to read directory").flatten() {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension();
            if let Some(ext) = ext {
                let ext = ext
                    .to_str()
                    .context("Failed to extract file extension as string")?;
                let ext = ext.to_ascii_lowercase();
                if ext == "e57" {
                    res.push(
                        path.to_str()
                            .context("Failed to convert path to string")?
                            .to_string(),
                    );
                }
            }
        } else if path.is_dir() {
            let mut files = list_files(&path)?;
            res.append(&mut files);
        }
    }
    Ok(res)
}

fn check_files(files: &[String]) -> bool {
    let mut all_ok = true;
    for f in files {
        if !check_file(f) {
            all_ok = false;
        }
    }
    all_ok
}

fn check_file(file_str: &str) -> bool {
    match File::open(file_str) {
        Ok(file) => match E57Reader::validate_crc(BufReader::new(file)) {
            Ok(_) => {
                println!("Validated file '{file_str}' successfully");
                true
            }
            Err(err) => {
                eprintln!("Failed to validate file '{file_str}': {err:#}");
                false
            }
        },
        Err(err) => {
            eprintln!("Failed to validate file '{file_str}': Failed to open file: {err:#}");
            false
        }
    }
}
