/*
 * Small example that can convert point clouds from XYZ into E57 files.
 *
 * The output file name will be the input file name plus ".e57".
 * The values in the input file need to be separated by spaces.
 * The first three values in each line must be X, Y and Z (as floating point values)
 * and last three values must be integers between 0 and 255 for red, green and blue.
 * Any additional columns will be ignored.
 *
 * X, Y and Z will be stored as Cartesian coordinates with 32 floarting points bits for each component.
 * RGB colors will be stored with 8 integer bits for each component.
 */

use anyhow::{ensure, Context, Result};
use e57::{E57Writer, Record, RecordValue};
use std::env::args;
use std::fs::File;
use std::io::{BufRead, BufReader};
use uuid::Uuid;

fn main() -> Result<()> {
    let args: Vec<String> = args().collect();
    ensure!(args.len() >= 2, "Usage: xyz-to-e57 <path/to/my.xyz>");

    let in_file = args[1].clone();
    let out_file = in_file.clone() + ".e57";

    let file = File::open(in_file).context("Failed to open XYZ file")?;
    let mut reader = BufReader::new(file);

    let file_guid = Uuid::new_v4().to_string();
    let mut e57_writer = E57Writer::from_file(out_file, &file_guid)
        .context("Unable to open E57 output file for writing")?;

    let pc_guid = Uuid::new_v4().to_string();
    let prototype = vec![
        Record::CARTESIAN_X_F32,
        Record::CARTESIAN_Y_F32,
        Record::CARTESIAN_Z_F32,
        Record::COLOR_RED_U8,
        Record::COLOR_GREEN_U8,
        Record::COLOR_BLUE_U8,
    ];
    let mut pc_writer = e57_writer
        .add_pointcloud(&pc_guid, prototype)
        .context("Failed to create point cloud writer")?;

    let mut line = String::new();
    while reader
        .read_line(&mut line)
        .context("Failed to read line from XYZ file")?
        > 0
    {
        let parts: Vec<&str> = line.trim().split(' ').collect();
        if parts.len() >= 6 {
            // Parse XYZ ASCII data
            let x: f32 = parts[0].parse().context("Failed to parse X value")?;
            let y: f32 = parts[1].parse().context("Failed to parse Y value")?;
            let z: f32 = parts[2].parse().context("Failed to parse Z value")?;
            let r: u8 = parts[3].parse().context("Failed to parse red value")?;
            let g: u8 = parts[4].parse().context("Failed to parse green value")?;
            let b: u8 = parts[5].parse().context("Failed to parse blue value")?;

            // Create E57 point for inserting
            let point = vec![
                RecordValue::Single(x),
                RecordValue::Single(y),
                RecordValue::Single(z),
                RecordValue::Integer(r as i64),
                RecordValue::Integer(g as i64),
                RecordValue::Integer(b as i64),
            ];
            pc_writer
                .add_point(point)
                .context("Failed to add E57 point")?;
        }
        line.clear();
    }

    pc_writer
        .finalize()
        .context("Failed to finalize point cloud in E57 file")?;

    e57_writer
        .finalize()
        .context("Failed to finalize E57 file")?;

    Ok(())
}
