/*
 * Small example application that will extract all images and their descriptions from an E57 file.
 */

use anyhow::{bail, Context, Result};
use e57::{E57Reader, Representation};
use std::fs::{write, File};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        bail!("Usage: extract-images <path/to/my.e57>");
    }

    // Open E57 and extract image metadata
    let mut e57 = E57Reader::from_file(&args[1]).context("Failed to open E57 file")?;
    let images = e57.images();
    let image_count = images.len();
    println!("Found {image_count} image(s) in the E57 file");

    for (index, img) in images.iter().enumerate() {
        // Extract metadata and write to txt file
        let desc = format!("{img:#?}");
        write(format!("image_{index}.txt"), &desc)
            .context("Failed to write description of image")?;

        // Extract preview image, if available
        if let Some(preview) = &img.visual_reference {
            let ext = format!("{:?}", preview.blob.format).to_lowercase();
            let filename = format!("image_{index}_preview.{ext}");
            let mut file = File::create(filename).unwrap();
            let size = e57.blob(&preview.blob.data, &mut file).unwrap();
            println!("Exported preview image with {size} bytes");

            // Extract preview mask, if available
            if let Some(blob) = &preview.mask {
                let filename = format!("image_{index}_preview_mask.png");
                let mut file = File::create(filename).unwrap();
                let size = e57.blob(blob, &mut file).unwrap();
                println!("Exported preview image mask with {size} bytes");
            }
        }

        // Extract projectable image, if available
        if let Some(rep) = &img.representation {
            let (blob, mask, type_name) = match rep {
                Representation::Pinhole(rep) => (&rep.blob, &rep.mask, "pinhole"),
                Representation::Spherical(rep) => (&rep.blob, &rep.mask, "spherical"),
                Representation::Cylindrical(rep) => (&rep.blob, &rep.mask, "cylindrical"),
            };
            let ext = format!("{:?}", blob.format).to_lowercase();
            let filename = format!("image_{index}_{type_name}.{ext}");
            let mut file = File::create(filename).unwrap();
            let size = e57.blob(&blob.data, &mut file).unwrap();
            println!("Exported {type_name} image with {size} bytes");

            // Extract preview mask, if available
            if let Some(blob) = mask {
                let filename = format!("image_{index}_preview_mask.png");
                let mut file = File::create(filename).unwrap();
                let size = e57.blob(blob, &mut file).unwrap();
                println!("Exported {type_name} image mask with {size} bytes");
            }
        }
    }

    Ok(())
}
