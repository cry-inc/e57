/*
 * Small example application that can export the first point cloud
 * in any E57 file as XYZ ASCII point cloud.
 *
 * The output file name will be the input file + ".xyz".
 * The values in the output file will be separated by a space as separator.
 * Spherical coordinates are converted automatically to cartesian coordinates.
 * Invalid coordinates (cartesian or spherical) will be skipped.
 * If there is no RGB color, it will try to use the intensity as gray RGB values.
 */

use anyhow::{bail, Context, Result};
use e57::E57Reader;
use e57::Point;
use std::env::args;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

fn main() -> Result<()> {
    let args: Vec<String> = args().collect();
    if args.len() < 2 {
        bail!("Usage: e57-to-xyz <path/to/my.e57>");
    }

    let in_file = args[1].clone();
    let out_file = in_file.clone() + ".xyz";

    let mut file = E57Reader::from_file(in_file).context("Failed to open E57 file")?;
    let pc = file
        .pointclouds()
        .first()
        .context("Unable to find point cloud in E57 file")?
        .clone();

    let writer = File::create(out_file).context("Unable to open output file for writing")?;
    let mut writer = BufWriter::new(writer);
    let iter = file
        .pointcloud(&pc)
        .context("Unable to get point cloud iterator")?;
    for p in iter {
        let p = p.context("Unable to read next point")?;
        let p = Point::from_values(p, &pc.prototype)
            .context("Failed to convert raw point to simple point")?;
        if let Some(c) = p.cartesian {
            if let Some(invalid) = p.cartesian_invalid {
                if invalid != 0 {
                    continue;
                }
            }
            writer
                .write_fmt(format_args!("{} {} {}", c.x, c.y, c.z))
                .context("Failed to write XYZ coordinates")?;
        } else if let Some(s) = p.spherical {
            if let Some(invalid) = p.spherical_invalid {
                if invalid != 0 {
                    continue;
                }
            }
            let cos_ele = f64::cos(s.elevation);
            let x = s.range * cos_ele * f64::cos(s.azimuth);
            let y = s.range * cos_ele * f64::sin(s.azimuth);
            let z = s.range * f64::sin(s.elevation);
            writer
                .write_fmt(format_args!("{x} {y} {z}"))
                .context("Failed to write XYZ coordinates after conversion")?;
        }
        if let Some(color) = p.color {
            writer
                .write_fmt(format_args!(
                    " {} {} {}",
                    (color.red * 255.) as u8,
                    (color.green * 255.) as u8,
                    (color.blue * 255.) as u8
                ))
                .context("Failed to write RGB color")?;
        } else if let Some(intensity) = p.intensity {
            writer
                .write_fmt(format_args!(
                    " {} {} {}",
                    (intensity * 255.) as u8,
                    (intensity * 255.) as u8,
                    (intensity * 255.) as u8
                ))
                .context("Failed to write intensity color")?;
        }
        writer
            .write_fmt(format_args!("\n"))
            .context("Failed to write newline")?;
    }

    Ok(())
}
