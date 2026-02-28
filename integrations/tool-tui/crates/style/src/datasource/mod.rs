//! Data source module for dx-style
//!
//! Provides file I/O utilities for reading HTML and other input files.

use std::fs;
use std::io;
use std::path::Path;

pub fn read_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    fs::read(path)
}
