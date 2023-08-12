/*
 * Small example application that can export all point clouds
 * from any E57 file as single merged XYZ ASCII point cloud.
 *
 * The output file name will be the input file plus ".xyz".
 * The values in the output file will be separated by a space as separator.
 *
 * Spherical coordinates are converted automatically to Cartesian coordinates.
 * Invalid coordinates will be skipped.
 * If there is no RGB color, it will try to use the intensity as grayscale RGB values.
 */

use anyhow::{bail, Context, Result};
use e57::E57Reader;
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
        // Iterate over all points in point cloud
        let mut iter = file
            .pointcloud_simple(&pointcloud)
            .context("Unable to get point cloud iterator")?;
        iter.convert_spherical(true);
        iter.skip_invalid(true);
        iter.apply_pose(true);
        for p in iter {
            let p = p.context("Unable to read next point")?;

            // Write XYZ data to output file
            writer
                .write_fmt(format_args!(
                    "{} {} {}",
                    p.cartesian.x, p.cartesian.y, p.cartesian.z
                ))
                .context("Failed to write XYZ coordinates")?;

            // If available, write RGB color or intensity color values
            if p.color_invalid == 0 {
                writer
                    .write_fmt(format_args!(
                        " {} {} {}",
                        (p.color.red * 255.) as u8,
                        (p.color.green * 255.) as u8,
                        (p.color.blue * 255.) as u8
                    ))
                    .context("Failed to write RGB color")?;
            } else if p.intensity_invalid == 0 {
                writer
                    .write_fmt(format_args!(
                        " {} {} {}",
                        (p.intensity * 255.) as u8,
                        (p.intensity * 255.) as u8,
                        (p.intensity * 255.) as u8
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
