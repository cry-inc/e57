# Changelog

All notable changes to this project will be documented in this file.

## [UNRELEASED]

- Breaking Fix: Made GUIDs for point clouds and images optional.
  This required changes in the corresponding public structs.
  The spec says the GUID for both is required, but the C++ implementation allows to omit it.
  Some software (e.g. Matterport) is generating files without them,
  so we need to make it optional to stay compatible and read these files.
  When creating E57 files, the library still enforces setting the GUIDs.

## [0.9.1] - 2023-09-11

- Fixed major bug that prevented adding images to E57 files.
  Some required property structs were accidentally private.
- Added some C++ utility code to generate test example files using the libE57format library.
- Restructured and extended integration tests to cover more cases.

## [0.9.0] - 2023-08-30

- Breaking Fix: Added missing implementation for offset in scaled integers.
  This required changes in the basic enum for record data types.
- Additional perfomance improvements when reading E57 files.
- Added validation for XML namespaces and attributes when writing E57 files with extensions.
- Added support for optional faster external CRC32 crate.
- Implemented optional size_hint() for reading point cloud iterators.
- Reworked image extraction tool to become a generic E57 unpack tool.
- Very minor improvements to the XYZ-to-E57 tool.

## [0.8.0] - 2023-08-22

- Breaking: Reworked simple iterator to make it easier to use
- Breaking: Removed simple iterator option to skip invalid points
- Speed up reading E57 files by ~30%
- Added convenience helper functions for point cloud struct
  to easily check if it has certain point attributes
- Added simple iterator option to convert Cartesian to spherical coordinates
- Added new E57-to-LAZ example tool
- Faster E57-to-XYZ tool (uses now ryu for float-to-string conversion)
- Added this CHANGELOG.md file

## [0.7.0] - 2023-08-16

- Breaking: Extended RecordName enum and made it non_exhaustive
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
