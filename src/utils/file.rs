use anyhow::Context;
use std::{fs, io::Read, path::Path};

pub(crate) fn read_file_bytes<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<u8>> {
    let mut file = fs::File::open(&path).context("open file")?;
    let mut content = Vec::new();
    file.read_to_end(&mut content).context("read file")?;

    Ok(content)
}
