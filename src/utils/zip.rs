use anyhow::{bail, Result};
use std::io::{Cursor, Read};
use zip::ZipArchive;

pub fn read_zip(data: Vec<u8>) -> Result<Vec<u8>> {
    let mut archive = ZipArchive::new(Cursor::new(data))?;

    if archive.len() != 1 {
        bail!(
            "We expected one file but the zip file contains {} files",
            archive.len()
        );
    }

    let mut file = archive.by_index(0)?;
    let mut contents = Vec::with_capacity(file.size() as usize);

    file.read_exact(&mut contents)?;

    Ok(contents)
}

#[allow(dead_code)]
pub fn read_zip_str(data: Vec<u8>) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(data))?;

    if archive.len() != 1 {
        bail!(
            "We expected one file but the zip file contains {} files",
            archive.len()
        );
    }

    let mut file = archive.by_index(0)?;
    let mut contents = String::new();
    contents.reserve(file.size() as usize);
    file.read_to_string(&mut contents)?;

    Ok(contents)
}
