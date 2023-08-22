# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- Breaking: Reworked simple iterator to make it easier to use
- Breaking: Removed simple iterator option to skip invalid points
- Speed up reading E57 files by ~30%
- Added convenience helper functions for point cloud struct
  to easily check if it has certain point attributes
- Added simple iterator option to convert Cartesian to spherical coordinates
- Added new E57-to-LAZ example tool
- Added this CHANGELOG.md file

## [0.7.0] - 2023-08-16

- Breaking: Extended RecordName enum and made in non_exhaustive
- Added missing support for point attribute extensions
- Optimized simple iterator to be ~30% faster

## [0.6.0] - 2023-08-12

- Breaking: Renamed some image structs and enums
- Breaking: Renamed point cloud iterator interface
- Breaking: Simplified Point struct and removed options
- Breaking: Removed Point constructor from raw values
- Added missing feature to add/write images in E57 files
- Added new simple point cloud iterator with some useful options
  to apply pose, skip invalid points, convert spherical to Cartesian
  coordinates and convert intensity to color.
- E57 to XYZ tool now respects and includes poses
- E57 to XYZ tool now reads all point clouds of the input file

## [0.5.1] - 2023-07-10

- Fix: Allow empty translation and rotation for poses

## [0.5.0] - 2023-05-07

- Breaking: Refactored some Record related prototype types
- Breaking: Removed simple XYZ RGB writing interface
- Added generic E57 point cloud writing for arbitrary point attributes
- Set optional XML root element metadata when writing
- Set optional point cloud metadata when writing

## [0.4.0] - 2023-03-26

- Breaking: Renamed E57 struct to E57Reader
- Added basic E57 writing support for XYZ RGB point clouds

## [0.3.1] - 2023-03-18

- Added extract images example tool
- Minor documentation improvements

## [0.3.0] - 2023-03-18

- Breaking: Fixed some typos in coordinate struct names
- Breaking: Changed CRC validation interface
- Breaking: Changed XML extraction interface
- Added functionality to read images from E57 files
- Use buffered reader for faster E57 file reading
- Added XML-extractor as example code
- Added E57-to-XYZ converter as example code
- Added CRC-validator as example code
