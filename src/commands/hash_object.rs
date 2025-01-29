use crate::objects::Object;
use anyhow::Context;
use std::{fs, io::Read, path::PathBuf};

pub(crate) fn invoke(write: bool, file_path: PathBuf) -> anyhow::Result<()> {
    // read file
    let mut file = fs::File::open(&file_path).context("open file")?;
    let mut content = Vec::new();
    let size = file.read_to_end(&mut content).context("read file")?;

    // compute object hash
    let hash = Object::compute_hash(&content, size)
        .with_context(|| format!("failed to compute hash of `{file_path:?}`"))?;
    let hash = hex::encode(hash);

    if write {
        Object::write(&hash, &content, size)
            .with_context(|| format!("failed to write object of `{file_path:?}`"))?;
    }

    println!("{}", hash);

    Ok(())
}
