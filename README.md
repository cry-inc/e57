# E57 Rust Library
[![Build Status](https://github.com/cry-inc/e57/workflows/CI/badge.svg)](https://github.com/cry-inc/e57/actions)
[![Crates.io](https://img.shields.io/crates/v/e57.svg)](https://crates.io/crates/e57)
[![Documentation](https://docs.rs/e57/badge.svg)](https://docs.rs/e57)
[![No Unsafe](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Dependencies](https://deps.rs/repo/github/cry-inc/e57/status.svg)](https://deps.rs/repo/github/cry-inc/e57)

A pure Rust library for reading E57 files. No unsafe code, no bloaty dependencies.

The scope is for now limited to reading E57 files, but this might change in the future.

## Please report incompatible files!
If you found an E57 file that works with other software but produces an error with this crate,
please let me know and create an issue on Github.
I want this library to work with as many files as possible.

Ideally, you can provide a link to the file itself. If that is not possible,
please include the full error message and the name of the software that produced the file.
If possible, please also include the XML section of the file.

## Motivation
The E57 file format is well established for exchanging data produced by terrestrial lasers scanners.
However, there are not many implementations that can read and write this file format.
Most applications use the original C++ reference implementation (see http://www.libe57.org/)
or the well maintained fork from Andy Maloney (see https://github.com/asmaloney/libE57Format).

I thought it would be nice to have a pure Rust solution without any unsafe code.
In my oppinion Rust is an excellent choice for parsers of untrusted data,
especially if you plan to use the code in something like a cloud backend.

When you want to handle E57 files inside a Rust project this crate will also avoid
all the issues that come with integrating C++ code.
