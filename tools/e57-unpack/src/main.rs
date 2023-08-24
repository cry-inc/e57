/*
 * Small example application that will try to "unpack" an E57 file.
 *
 * It will create an XML file for the full original metadata,
 * a CSV file with the raw values for all point clouds,
 * all images will be extracted as individual files and
 * the parsed pieces of metadata will be stored as text files.
 *
 * The CSV files will use a semicolon as separator and Unix line endings.
 * The first line of the CSV file contains the names and types of the columns.
 *
 * The unpacked results will be saved into an folder with the suffix "_unpacked"
 * in the same folder as the original file.
 */

use anyhow::{bail, Context, Result};
use e57::{DateTime, E57Reader, Extension, Header, Projection, RecordValue};
use std::fs::{create_dir_all, write, File};
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Debug)]
pub struct E57Metadata {
    pub header: Header,
    pub format_name: String,
    pub guid: String,
    pub extensions: Vec<Extension>,
    pub creation: Option<DateTime>,
    pub coordinate_metadata: Option<String>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        bail!("Usage: extract-images <path/to/my.e57>");
    }

    // Prepare input file and output folder
    let input_file = &args[1];
    let output_folder = input_file.to_owned() + "_unpacked";
    let output_folder = Path::new(&output_folder);
    create_dir_all(output_folder).context("Failed to create output folder")?;

    // Open E57 file
    let mut e57 = E57Reader::from_file(input_file).context("Failed to open E57 file")?;

    // Extract XML section
    let xml = e57.xml();
    let xml_file = output_folder.join("metadata.xml");
    write(xml_file, xml).context("Failed to write XML metadata")?;
    println!("Finished extracting XML data");

    // Extract parsed metadata
    let metadata = E57Metadata {
        header: e57.header(),
        format_name: e57.format_name().to_owned(),
        guid: e57.guid().to_owned(),
        extensions: e57.extensions(),
        creation: e57.creation(),
        coordinate_metadata: e57.coordinate_metadata().map(|cm| cm.to_owned()),
    };
    let metadata_file = output_folder.join("metadata.txt");
    let desc = format!("{metadata:#?}");
    write(metadata_file, &desc).context("Failed to write parsed metadata of E57")?;
    println!("Finished writing parsed metadata");

    // Extract images
    let images = e57.images();
    let image_count = images.len();
    println!("Found {image_count} image(s)");
    for (index, img) in images.iter().enumerate() {
        println!("Starting to extract data for image #{index}...");

        // Extract metadata and write to txt file
        let img_metadata_file = output_folder.join(format!("image_{index}.txt"));
        let desc = format!("{img:#?}");
        write(img_metadata_file, &desc).context("Failed to write metadata of image")?;
        println!("  Exported image metadata");

        // Extract preview image, if available
        if let Some(preview) = &img.visual_reference {
            let ext = format!("{:?}", preview.blob.format).to_lowercase();
            let file_name = format!("image_{index}_preview.{ext}");
            let file_path = output_folder.join(file_name);
            let file =
                File::create(file_path).context("Failed to open preview image file for writing")?;
            let mut writer = BufWriter::new(file);
            let size = e57
                .blob(&preview.blob.data, &mut writer)
                .context("Failed to write preview image blob")?;
            println!("  Exported preview image with {size} bytes");

            // Extract preview mask, if available
            if let Some(blob) = &preview.mask {
                let file_name = format!("image_{index}_preview_mask.png");
                let file_path = output_folder.join(file_name);
                let file = File::create(file_path)
                    .context("Failed to open preview mask image file for writing")?;
                let mut writer = BufWriter::new(file);
                let size = e57
                    .blob(blob, &mut writer)
                    .context("Failed to write preview mask image blob")?;
                println!("  Exported preview image mask with {size} bytes");
            }
        }

        // Extract projectable image, if available
        if let Some(rep) = &img.projection {
            let (blob, mask, type_name) = match rep {
                Projection::Pinhole(rep) => (&rep.blob, &rep.mask, "pinhole"),
                Projection::Spherical(rep) => (&rep.blob, &rep.mask, "spherical"),
                Projection::Cylindrical(rep) => (&rep.blob, &rep.mask, "cylindrical"),
            };
            let ext = format!("{:?}", blob.format).to_lowercase();
            let file_name = format!("image_{index}_{type_name}.{ext}");
            let file_path = output_folder.join(file_name);
            let file = File::create(file_path).context("Failed to open image file for writing")?;
            let mut writer = BufWriter::new(file);
            let size = e57
                .blob(&blob.data, &mut writer)
                .context("Failed to write image blob")?;
            println!("Exported {type_name} image with {size} bytes");

            // Extract mask, if available
            if let Some(blob) = mask {
                let file_name = format!("image_{index}_{type_name}_mask.png");
                let file_path = output_folder.join(file_name);
                let file = File::create(file_path)
                    .context("Failed to open mask image file for writing")?;
                let mut writer = BufWriter::new(file);
                let size = e57
                    .blob(blob, &mut writer)
                    .context("Failed to write mask image blob")?;
                println!("  Exported image mask with {size} bytes");
            }
        }
    }

    // Extract point clouds
    let pointclouds = e57.pointclouds();
    let pc_count = pointclouds.len();
    println!("Found {pc_count} point cloud(s)");
    for (index, pc) in pointclouds.iter().enumerate() {
        println!("Starting to extract data for point cloud #{index}...");

        // Extract metadata and write to txt file
        let pc_metadata_file = output_folder.join(format!("pc_{index}.txt"));
        let desc = format!("{pc:#?}");
        write(pc_metadata_file, &desc).context("Failed to write metadata of point cloud")?;
        println!("  Exported point cloud metadata");

        // Create CSV header
        let file_name = format!("pc_{index}.csv");
        let file_path = output_folder.join(file_name);
        let file =
            File::create(file_path).context("Failed to open point cloud file for writing")?;
        let mut writer = BufWriter::new(file);
        let headers: Vec<String> = pc
            .prototype
            .iter()
            .map(|r| format!("{:?} {:?}", r.name, r.data_type))
            .collect();
        let mut header = headers.join(";");
        header += "\n";
        writer
            .write_all(header.as_bytes())
            .context("Failed to write CSV header")?;

        // Write CSV data
        let iter = e57
            .pointcloud_raw(pc)
            .context("Failed to open point cloud iterator")?;
        for p in iter {
            let p = p.context("Failed to extract raw point")?;
            let values: Vec<String> = p
                .iter()
                .map(|r| match &r {
                    RecordValue::Single(s) => s.to_string(),
                    RecordValue::Double(d) => d.to_string(),
                    RecordValue::ScaledInteger(si) => si.to_string(),
                    RecordValue::Integer(i) => i.to_string(),
                })
                .collect();
            let line = values.join(";") + "\n";
            writer
                .write_all(line.as_bytes())
                .context("Failed to write CSV point")?;
        }
        println!("  Exported point cloud data to CSV file");
    }

    Ok(())
}
