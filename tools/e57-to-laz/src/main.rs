/*
 * Small example application that can convert all point clouds
 * from one or more E57 files into a single merged LAZ 1.4 file.
 *
 * If the first argument is a file path, it will read a single E57 file.
 * If it is a directory, it will read all E57 files in that directory recursively.
 *
 * The second argument is the LAZ output file.
 * The third argument is an optional scale for the LAZ output file.
 * The default scale is 0.0001.
 *
 * Spherical coordinates are converted automatically to Cartesian coordinates.
 * Invalid and incomplete coordinates will be skipped.
 * The poses of different point clouds in the E57 will be applied.
 */

use anyhow::{bail, ensure, Context, Result};
use e57::{CartesianCoordinate, E57Reader};
use las::{Builder, Color, Point, Transform, Vector, Version, Writer};
use std::env::args;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    // Check command line arguments and show usage
    let args: Vec<String> = args().collect();
    ensure!(
        args.len() >= 2,
        "Usage:\n  e57-to-laz <path/to/in.e57> <path/to/out.laz>\n  e57-to-laz <path/to/in/folder/> <path/to/out.laz> <optional-las-scale>"
    );

    // Prepare input and output file paths
    let in_path_str = &args[1];
    let in_path = Path::new(in_path_str);
    println!("Input: {in_path_str}");
    ensure!(in_path.exists(), "The path '{in_path_str}' does not exist");
    let in_paths = if in_path.is_dir() {
        list_e57_files(in_path).context("Failed to list E57 files")?
    } else if in_path.is_file() {
        vec![in_path.to_path_buf()]
    } else {
        bail!("The path '{in_path_str}' does not point to a directory or a file");
    };
    let out_path_str = &args[2];
    let out_path = Path::new(out_path_str);
    println!("Output: {out_path_str}");

    // Determine scale factor for LAZ output file
    let scale = if args.len() > 3 {
        args[3]
            .parse::<f64>()
            .context("Failed to parse scale argument")?
    } else {
        0.0001
    };
    println!("Scale: {}", scale);

    println!(
        "Found {} E57 file(s), starting conversion...",
        in_paths.len()
    );

    // Build LAZ header
    let mut builder = Builder::from(Version::new(1, 4));
    builder.point_format.has_color = true;
    builder.point_format.is_compressed = true;
    builder.transforms = Vector {
        x: Transform { scale, offset: 0.0 },
        y: Transform { scale, offset: 0.0 },
        z: Transform { scale, offset: 0.0 },
    };
    let header = builder
        .into_header()
        .context("Failed to build LAZ header")?;

    // Prepare writing to output file
    let mut writer =
        Writer::from_path(out_path, header).context("Failed to open new LAZ file for writing")?;

    // Loop over all input files
    for (index, in_file) in in_paths.iter().enumerate() {
        println!(
            "Started reading E57 file {}/{}: {}",
            index + 1,
            in_paths.len(),
            in_file.display()
        );

        // Open E57 input file for reading
        let mut file = E57Reader::from_file(in_file).context("Failed to open E57 file")?;

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
                let mut point = Point::default();
                if let CartesianCoordinate::Valid { x, y, z } = p.cartesian {
                    point.x = x;
                    point.y = y;
                    point.z = z;
                } else {
                    continue;
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
                } else {
                    point.intensity = 0;
                }
                writer
                    .write_point(point)
                    .context("Failed to write LAZ point")?;
            }
        }
        println!("Finished reading E57 file {}/{}", index + 1, in_paths.len());
    }

    writer.close().context("Failed to close LAZ file")?;
    drop(writer);
    println!("Finished writing LAZ file {}", out_path.display());

    Ok(())
}

fn list_e57_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut res = Vec::new();
    for entry in path.read_dir().expect("Failed to read directory").flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext = ext
                    .to_str()
                    .context("Failed to extract file extension as string")?
                    .to_ascii_lowercase();
                if ext == "e57" {
                    res.push(path);
                }
            }
        } else if path.is_dir() {
            let mut files = list_e57_files(&path)?;
            res.append(&mut files);
        }
    }
    Ok(res)
}
