/*
 * Small example application that can convert all point clouds
 * from any E57 file into a single merged XYZ ASCII file.
 *
 * The output file name will be the input file name plus ".xyz".
 * The values in the output file will be separated by a space as separator.
 *
 * Spherical coordinates are converted automatically to Cartesian coordinates.
 * Invalid and incomplete coordinates will be skipped.
 * If there is no RGB color, it will use the intensity as grayscale RGB values.
 * If there is no color and no intensity, it will only write X, Y and Z values.
 */

use anyhow::{ensure, Context, Result};
use e57::{CartesianCoordinate, E57Reader};
use std::env::args;
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() -> Result<()> {
    // Check command line arguments and show usage
    let args: Vec<String> = args().collect();
    ensure!(args.len() >= 2, "Usage: e57-to-xyz <path/to/my.e57>");

    // Prepare input and output file paths
    let in_file = args[1].clone();
    let out_file = in_file.clone() + ".xyz";

    // Open E57 input file for reading
    let mut file = E57Reader::from_file(in_file).context("Failed to open E57 file")?;

    // Prepare buffered writing into output file
    let writer = File::create(out_file).context("Unable to open output file for writing")?;
    let mut writer = BufWriter::new(writer);

    // Prepare fast floating point to ASCII generation.
    // The std implementation is a bit slower compared to the specialized ryu crate.
    let mut buffer = ryu::Buffer::new();

    // Loop over all point clouds in the E57 file
    let pointclouds = file.pointclouds();
    for pointcloud in pointclouds {
        let mut iter = file
            .pointcloud_simple(&pointcloud)
            .context("Unable to get point cloud iterator")?;

        // Set point iterator options
        iter.spherical_to_cartesian(true);
        iter.cartesian_to_spherical(false);
        iter.intensity_to_color(true);
        iter.apply_pose(true);

        // Iterate over all points in point cloud
        for p in iter {
            let p = p.context("Unable to read next point")?;

            // Write XYZ data to output file
            if let CartesianCoordinate::Valid { x, y, z } = p.cartesian {
                let space = " ".as_bytes();
                let xyz_err = "Failed to write XYZ coordinates";

                let str = buffer.format(x);
                writer.write_all(str.as_bytes()).context(xyz_err)?;
                writer.write_all(space).context(xyz_err)?;

                let str = buffer.format(y);
                writer.write_all(str.as_bytes()).context(xyz_err)?;
                writer.write_all(space).context(xyz_err)?;

                let str = buffer.format(z);
                writer.write_all(str.as_bytes()).context(xyz_err)?;
            } else {
                continue;
            }

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
            }

            // Write new line before next point
            writer
                .write_all("\n".as_bytes())
                .context("Failed to write newline")?;
        }
    }

    Ok(())
}
