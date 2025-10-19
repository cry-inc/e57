/*
 * Small example application that can convert all point clouds
 * from any E57 file into a single merged LAZ file.
 *
 * The output file name will be the input file name plus ".laz".
 *
 * Spherical coordinates are converted automatically to Cartesian coordinates.
 * Invalid and incomplete coordinates will be skipped.
 */

use anyhow::{ensure, Context, Result};
use e57::{CartesianCoordinate, E57Reader};
use las::{Builder, Color, Point, Version, Writer};
use std::env::args;

fn main() -> Result<()> {
    // Check command line arguments and show usage
    let args: Vec<String> = args().collect();
    ensure!(args.len() >= 2, "Usage: e57-to-laz <path/to/my.e57>");

    // Prepare input and output file paths
    let in_file = args[1].clone();
    let out_file = in_file.clone() + ".laz";

    // Open E57 input file for reading
    let mut file = E57Reader::from_file(in_file).context("Failed to open E57 file")?;

    // Check if any of the input point clouds has color
    let has_color = file.pointclouds().iter().any(|pc| pc.has_color());

    // Build LAZ header
    let mut builder = Builder::from(Version::default());
    builder.point_format.has_color = has_color;
    builder.point_format.is_compressed = true;
    let header = builder
        .into_header()
        .context("Failed to build LAZ header")?;

    // Prepare writing to output file
    let mut writer =
        Writer::from_path(out_file, header).context("Failed to open new LAZ file for writing")?;

    // Loop over all point clouds in the E57 file
    let pointclouds = file.pointclouds();
    for (index, pointcloud) in pointclouds.iter().enumerate() {
        println!(
            "Converting point cloud {}/{}...",
            index + 1,
            pointclouds.len()
        );

        let mut iter = file
            .pointcloud_simple(pointcloud)
            .context("Unable to get point cloud iterator")?;

        // Set point iterator options
        iter.spherical_to_cartesian(true);
        iter.cartesian_to_spherical(false);
        iter.intensity_to_color(false);
        iter.apply_pose(true);

        // Count different point states
        let mut valid = 0;
        let mut direction_only = 0;
        let mut invalid = 0;

        // Iterate over all points in point cloud
        for p in iter {
            let p = p.context("Unable to read next point")?;
            let mut point = Point::default();
            match p.cartesian {
                CartesianCoordinate::Valid { x, y, z } => {
                    valid += 1;
                    point.x = x;
                    point.y = y;
                    point.z = z;
                }
                CartesianCoordinate::Direction { .. } => {
                    direction_only += 1;
                    continue;
                }
                CartesianCoordinate::Invalid => {
                    invalid += 1;
                    continue;
                }
            }
            if let Some(color) = p.color {
                point.color = Some(Color {
                    red: (color.red * u16::MAX as f32) as u16,
                    green: (color.green * u16::MAX as f32) as u16,
                    blue: (color.blue * u16::MAX as f32) as u16,
                })
            }
            if let Some(intensity) = p.intensity {
                point.intensity = (intensity * u16::MAX as f32) as u16;
            }
            writer
                .write_point(point)
                .context("Failed to write LAZ point")?;
        }

        // Print collected point states
        println!("Skipped {invalid} invalid points");
        println!("Skipped {direction_only} direction only points");
        println!("Converted {valid} valid points");

        println!(
            "Finished converting point cloud {}/{}",
            index + 1,
            pointclouds.len()
        );
    }

    Ok(())
}
