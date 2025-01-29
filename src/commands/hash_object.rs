use crate::{objects::Object, utils::file::read_file_bytes};
use anyhow::Context;
use std::path::PathBuf;

pub(crate) fn invoke(write: bool, file_path: PathBuf) -> anyhow::Result<()> {
    let file_bytes = read_file_bytes(&file_path)
        .with_context(|| format!("failed to read file `{file_path:?}`"))?;

    // compute object hash
    let hash = Object::compute_hash(&file_bytes)
        .with_context(|| format!("failed to compute hash of `{file_path:?}`"))?;
    let hash = hex::encode(hash);

    if write {
        Object::write(&hash, &file_bytes)
            .with_context(|| format!("failed to write object of `{file_path:?}`"))?;
    }

    println!("{}", hash);

    Ok(())
}
