/*
 * Example application that projects structured scans in E57 files to 360 degree spherical panorama PNG RGBA images.
 * By default the point color will be used, the intensity will be used as fallback.
 * Areas without color and intensity will stay transparent and black.
 * The origins of the scans will be the center of projection for the generated panorama images.
 * Horizontally the image will cover 360 degress and vertically 180 degrees.
 *
 * Important hint:
 * To get the existing PNG or JPEG spherical images stored in E57 files use the `e57-unpack` tool instead.
 *
 * The output files will be named like the input file and placed in the same folder.
 * They will have an additional number suffix and the extension PNG.
 *
 * You are just interested in the 2D row/column grid of the structured scan?
 * Use the `e57-to-image` tool instead!
 */

use anyhow::{ensure, Context, Result};
use e57::{E57Reader, SphericalCoordinate};
use png::Encoder;
use std::{
    env::args,
    f64::consts::{FRAC_PI_2, PI},
    fs::File,
    io::BufWriter,
    path::Path,
};

fn main() -> Result<()> {
    // Check command line arguments and show usage
    let args: Vec<String> = args().collect();
    ensure!(
        args.len() >= 2,
        "Usage: e57-to-pano <path/to/my.e57> [optional_image_width] [optional_image_height]"
    );

    // Prepare input file path
    let in_path = Path::new(&args[1]);

    // Check optional width and height
    let width = if args.len() >= 3 {
        let width = args[2].parse::<usize>().context("Failed to parse width")?;
        ensure!(width > 0);
        Some(width)
    } else {
        None
    };
    let height = if args.len() >= 4 {
        let height = args[3].parse::<usize>().context("Failed to parse height")?;
        ensure!(height > 0);
        Some(height)
    } else {
        None
    };

    // Open E57 input file for reading
    let mut file = E57Reader::from_file(in_path).context("Failed to open E57 file")?;

    // Loop over all point clouds in the E57 file
    let pointclouds = file.pointclouds();
    for (index, pointcloud) in pointclouds.iter().enumerate() {
        if !pointcloud.has_color() && !pointcloud.has_intensity() {
            println!("Point cloud #{index} has no color and no intensity, skipping...");
            continue;
        }

        if pointcloud.records < 1 {
            println!("Point cloud #{index} is empty, skipping...");
            continue;
        }

        if !pointcloud.has_row_column() && !pointcloud.has_spherical() {
            println!("Warning: Point cloud #{index} has no row/column indices and no spherical coordinates, it might be unstructured!");
        }

        // Determine width and height of image
        let calc_height = (((pointcloud.records as f32) * 2.0).sqrt() / 2.0) as usize;
        let width = width.unwrap_or(calc_height * 2);
        let height = height.unwrap_or(calc_height);
        println!("Point cloud #{index} image size: {width}x{height}");

        // Allocate memory for output image RGBA buffer
        // Default color for all pixels is black and transparent!
        let mut buffer = vec![0_u8; width * height * 4];

        // Loop over all points to project the points into the panorama
        let mut iter = file
            .pointcloud_simple(pointcloud)
            .context("Unable to get simple point cloud iterator")?;
        iter.cartesian_to_spherical(true); // We need spherical coordinates for the projection!
        for p in iter {
            let p = p.context("Unable to read next point")?;

            // Get RGB value of the point
            let rgb = if let Some(color) = p.color {
                [
                    (color.red * 255.0) as u8,
                    (color.green * 255.0) as u8,
                    (color.blue * 255.0) as u8,
                ]
            } else if let Some(intensity) = p.intensity {
                [
                    (intensity * 255.0) as u8,
                    (intensity * 255.0) as u8,
                    (intensity * 255.0) as u8,
                ]
            } else {
                // Individual points might have no color or intensity.
                // Leave them at the default color!
                continue;
            };

            // Get angles from spherical coordinates
            let (mut az, mut el) = match p.spherical {
                SphericalCoordinate::Valid {
                    azimuth, elevation, ..
                } => (azimuth, elevation),
                SphericalCoordinate::Direction { azimuth, elevation } => (azimuth, elevation),
                SphericalCoordinate::Invalid => continue, // Nothing to project
            };

            // Make sure the angles are in the expected range
            const TWO_PI: f64 = PI * 2.0;
            while az <= -PI {
                az += TWO_PI;
            }
            while az > PI {
                az -= TWO_PI;
            }
            while el <= -FRAC_PI_2 {
                el += PI;
            }
            while el > FRAC_PI_2 {
                el -= PI;
            }

            // Get X and Y coordinates in panorama image from angles
            let az_normalized = (az + PI) / TWO_PI;
            let x = (az_normalized * width as f64).clamp(0.0, (width - 1) as f64) as usize;
            let el_normalized = (el + FRAC_PI_2) / PI;
            let y = (el_normalized * height as f64).clamp(0.0, (height - 1) as f64) as usize;
            let x = width - x - 1; // Prevent image from being horizontally mirrored
            let y = height - y - 1; // Prevent image from being upside down

            // Set pixel color
            let offset = y * width * 4 + x * 4;
            buffer[offset] = rgb[0];
            buffer[offset + 1] = rgb[1];
            buffer[offset + 2] = rgb[2];
            buffer[offset + 3] = 255; // Set alpha to opaque
        }

        // Prepare output file name
        let out_path = args[1].clone() + &format!(".{index}.png");

        // Write PNG file
        let out_file = File::create(&out_path).context("Unable to open output file")?;
        let writer = BufWriter::new(out_file);
        let mut encoder = Encoder::new(writer, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .context("Failed to write PNG header")?;
        writer
            .write_image_data(&buffer)
            .context("Failed to write PNG data")?;

        println!("Exported panorama for point cloud #{index} to {out_path}");
    }

    Ok(())
}
