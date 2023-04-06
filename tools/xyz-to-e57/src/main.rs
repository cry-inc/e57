/*
 * Small example that can convert point clouds from XYZ into E57 files.
 *
 * The output file name will be the input file name plus ".e57".
 * The values in the input file need to be separated by spaces.
 * The first three values in each line must be X, Y and Z (as floating point values)
 * and last three values must be integers between 0 and 255 for red, green and blue.
 * Any additional columns will be ignored.
 */

use anyhow::{bail, Context, Result};
use e57::{E57Writer, RawPoint, Record, RecordDataType, RecordName};
use std::env::args;
use std::fs::File;
use std::io::{BufRead, BufReader};
use uuid::Uuid;

fn main() -> Result<()> {
    let args: Vec<String> = args().collect();
    if args.len() < 2 {
        bail!("Usage: xyz-to-e57 <path/to/my.xyz>");
    }

    let in_file = args[1].clone();
    let out_file = in_file.clone() + ".e57";

    let file = File::open(in_file).context("Failed to open XYZ file")?;
    let mut reader = BufReader::new(file);
    let mut e57_writer =
        E57Writer::from_file(out_file).context("Unable to open E57 output file for writing")?;

    let pc_guid = Uuid::new_v4().to_string();
    let xyz_type = RecordDataType::Double {
        min: None,
        max: None,
    };
    let rgb_type = RecordDataType::Integer { min: 0, max: 255 };
    let prototype = vec![
        Record {
            name: RecordName::CartesianX,
            data_type: xyz_type.clone(),
        },
        Record {
            name: RecordName::CartesianY,
            data_type: xyz_type.clone(),
        },
        Record {
            name: RecordName::CartesianZ,
            data_type: xyz_type,
        },
        Record {
            name: RecordName::ColorRed,
            data_type: rgb_type.clone(),
        },
        Record {
            name: RecordName::ColorGreen,
            data_type: rgb_type.clone(),
        },
        Record {
            name: RecordName::ColorBlue,
            data_type: rgb_type,
        },
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
            let x: f64 = parts[0].parse().context("Failed to parse X value")?;
            let y: f64 = parts[1].parse().context("Failed to parse X value")?;
            let z: f64 = parts[2].parse().context("Failed to parse X value")?;
            let r: u8 = parts[3].parse().context("Failed to parse red value")?;
            let g: u8 = parts[4].parse().context("Failed to parse red value")?;
            let b: u8 = parts[5].parse().context("Failed to parse red value")?;

            // Create E57 point for inserting
            let mut point = RawPoint::new();
            point.insert(RecordName::CartesianX, e57::RecordValue::Double(x));
            point.insert(RecordName::CartesianY, e57::RecordValue::Double(y));
            point.insert(RecordName::CartesianZ, e57::RecordValue::Double(z));
            point.insert(RecordName::ColorRed, e57::RecordValue::Integer(r as i64));
            point.insert(RecordName::ColorGreen, e57::RecordValue::Integer(g as i64));
            point.insert(RecordName::ColorBlue, e57::RecordValue::Integer(b as i64));
            pc_writer
                .add_point(point)
                .context("Failed to add E57 point")?;
        }
        line.clear();
    }

    pc_writer
        .finalize()
        .context("Failed to finalize point cloud in E57 file")?;

    let e57_guid = Uuid::new_v4().to_string();
    e57_writer
        .finalize(&e57_guid)
        .context("Failed to finalize E57 file")?;

    Ok(())
}
