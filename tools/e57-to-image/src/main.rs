/*
 * Small example application that converts structured scans in E57 files to planar PNG RGBA images.
 * This works only for structured scans with row and column indices.
 * By default the point color will be used, the intensity will be used as fallback.
 * Areas without color and intensity will stay transparent and black.
 *
 * Important hint:
 * To get the existing PNG or JPEG images stored in E57 files use the `e57-unpack` tool instead.
 *
 * The output files will be named like the input file and placed in the same folder.
 * They will have an additional number suffix and the extension PNG.
 *
 * Please note that the output picture *must not* be treated as 360 degree panorama image.
 * It just visualizes the 2D row/column grid of the scans as is.
 * Use the tool `e57-to-pano` if you need valid 360 degree panorama images!
 */

use anyhow::{ensure, Context, Result};
use e57::E57Reader;
use png::Encoder;
use std::{env::args, fs::File, io::BufWriter, path::Path};

fn main() -> Result<()> {
    // Check command line arguments and show usage
    let args: Vec<String> = args().collect();
    ensure!(args.len() >= 2, "Usage: e57-to-image <path/to/my.e57>");

    // Prepare input file path
    let in_path = Path::new(&args[1]);

    // Open E57 input file for reading
    let mut file = E57Reader::from_file(in_path).context("Failed to open E57 file")?;

    // Loop over all point clouds in the E57 file
    let pointclouds = file.pointclouds();
    for (index, pointcloud) in pointclouds.iter().enumerate() {
        if !pointcloud.has_row_column() {
            println!("Point cloud #{index} has no row/column indices, skipping...");
            continue;
        }

        if !pointcloud.has_color() && !pointcloud.has_intensity() {
            println!("Point cloud #{index} has no color and no intensity, skipping...");
            continue;
        }

        if pointcloud.records < 1 {
            println!("Point cloud #{index} is empty, skipping...");
            continue;
        }

        // First loop over all points to determine image size
        let mut row_min = i64::MAX;
        let mut row_max = i64::MIN;
        let mut col_min = i64::MAX;
        let mut col_max = i64::MIN;
        let iter = file
            .pointcloud_simple(pointcloud)
            .context("Unable to get simple point cloud iterator")?;
        for p in iter {
            let p = p.context("Unable to read next point")?;
            if p.row < row_min {
                row_min = p.row;
            }
            if p.row > row_max {
                row_max = p.row;
            }
            if p.column < col_min {
                col_min = p.column;
            }
            if p.column > col_max {
                col_max = p.column;
            }
        }

        // Determine image size
        let width = col_max - col_min;
        println!("Point cloud #{index} image width: {width}");
        ensure!(width >= 0, "Column values have empty or negative width");
        let width = (width + 1) as usize;

        let height = row_max - row_min;
        println!("Point cloud #{index} image height: {height}");
        ensure!(height >= 0, "Row values have empty or negative height");
        let height = (height + 1) as usize;

        // Allocate memory for output image RGBA buffer
        // Default color for all pixels is black and transparent!
        let mut buffer = vec![0_u8; width * height * 4];

        // Second loop over all points to draw the image
        let iter = file
            .pointcloud_simple(pointcloud)
            .context("Unable to get simple point cloud iterator")?;
        for p in iter {
            let p = p.context("Unable to read next point")?;

            // Since there is a intensity to color fallback
            // we only need to ask for color here!
            let rgb = if let Some(color) = p.color {
                [
                    (color.red * 255.0) as u8,
                    (color.green * 255.0) as u8,
                    (color.blue * 255.0) as u8,
                ]
            } else {
                // Individual points might have no color and intensity.
                // Leave them at the default color!
                continue;
            };

            let x = (p.column - col_min) as usize;
            let y = (p.row - row_min) as usize;
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

        println!("Exported image for point cloud #{index} to {out_path}");
    }

    Ok(())
}
