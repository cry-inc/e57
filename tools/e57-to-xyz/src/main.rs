/*
 * Small example application that can export all point clouds
 * from any E57 file as single merged XYZ ASCII point cloud.
 *
 * The output file name will be the input file plus ".xyz".
 * The values in the output file will be separated by a space as separator.
 *
 * Spherical coordinates are converted automatically to cartesian coordinates.
 * Invalid coordinates (cartesian or spherical) will be skipped.
 * If there is no RGB color, it will try to use the intensity as grayscale RGB values.
 */

use anyhow::{bail, Context, Result};
use e57::{E57Reader, Point, Transform};
use nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3};
use std::env::args;
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() -> Result<()> {
    // Check command line arguments and show usage
    let args: Vec<String> = args().collect();
    if args.len() < 2 {
        bail!("Usage: e57-to-xyz <path/to/my.e57>");
    }

    // Prepare input and output file paths
    let in_file = args[1].clone();
    let out_file = in_file.clone() + ".xyz";

    // Open E57 input file for reading
    let mut file = E57Reader::from_file(in_file).context("Failed to open E57 file")?;

    // Prepare buffered writing into output file
    let writer = File::create(out_file).context("Unable to open output file for writing")?;
    let mut writer = BufWriter::new(writer);

    // Loop over all point clouds in the E57 file
    let pointclouds = file.pointclouds();
    for pointcloud in pointclouds {
        // Prepare rotation and translation data
        let transform = pointcloud.transform.clone().unwrap_or(Transform::default());
        let rotation = UnitQuaternion::from_quaternion(Quaternion::new(
            transform.rotation.w,
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
        ));
        let translation = Vector3::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        );

        // Iterate over all points in point cloud
        let iter = file
            .pointcloud(&pointcloud)
            .context("Unable to get point cloud iterator")?;
        for p in iter {
            let p = p.context("Unable to read next point")?;

            // Convert raw values to simple point data structure
            let p = Point::from_values(p, &pointcloud.prototype)
                .context("Failed to convert raw point to simple point")?;

            // Read cartesian or spherical points and convert to cartesian
            let xyz = if let Some(c) = p.cartesian {
                if let Some(invalid) = p.cartesian_invalid {
                    if invalid != 0 {
                        continue;
                    }
                }
                Point3::new(c.x, c.y, c.z)
            } else if let Some(s) = p.spherical {
                if let Some(invalid) = p.spherical_invalid {
                    if invalid != 0 {
                        continue;
                    }
                }
                let cos_ele = f64::cos(s.elevation);
                Point3::new(
                    s.range * cos_ele * f64::cos(s.azimuth),
                    s.range * cos_ele * f64::sin(s.azimuth),
                    s.range * f64::sin(s.elevation),
                )
            } else {
                // No coordinates found, skip point
                continue;
            };

            // Apply per point cloud transformation
            let xyz = rotation.transform_point(&xyz) + translation;

            // Write XYZ data to output file
            writer
                .write_fmt(format_args!("{} {} {}", xyz[0], xyz[1], xyz[2]))
                .context("Failed to write XYZ coordinates")?;

            // If available, write RGB color or intensity color values
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

            // Write new line before next point
            writer
                .write_fmt(format_args!("\n"))
                .context("Failed to write newline")?;
        }
    }

    Ok(())
}
