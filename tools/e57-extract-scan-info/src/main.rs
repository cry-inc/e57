/*
 * Small example application that extracts some metadata for
 * all scans/point clouds in the E57 file into a CSV file.
 *
 * The CSV will contain the following properties of each point cloud:
 * - GUID
 * - Name of the point cloud
 * - Number of points
 * - Position (translation part of the transform) as X,Y,Z
 * - Rotation quaternion of the transform as X,Y,Z,W
 *
 * There will be one line per point cloud and each value
 * is separated by an semicolon and ends with an Unix line break.
 *
 * The output file will be named like the input file plus `.csv` extension.
 */

use anyhow::{ensure, Context, Result};
use e57::{E57Reader, Quaternion, Translation};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() >= 2,
        "Usage: e57-extract-scan-info <path/to/my.e57>"
    );

    let infile = &args[1];
    let outfile = format!("{infile}.csv");

    let reader = E57Reader::from_file(infile).context("Failed to open E57 file")?;
    let mut csv_data = String::new();
    for pc in reader.pointclouds() {
        let guid = pc.guid.unwrap_or_default();
        let name = pc.name.unwrap_or_default();
        let points = pc.records;
        let transform = pc.transform.unwrap_or_default();
        let Translation { x, y, z } = transform.translation;
        let position = format!("{x},{y},{z}");
        let Quaternion { w, x, y, z } = transform.rotation;
        let rotation = format!("{x},{y},{z},{w}");
        let line = format!("{guid};{name};{points};{position};{rotation}\n");
        csv_data.push_str(&line);
    }

    std::fs::write(outfile, csv_data).context("Failed to write outputr CSV file")
}
