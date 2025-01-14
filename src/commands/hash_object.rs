use anyhow::Context;
use flate2::{write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

pub(crate) fn invoke(write: bool, file_path: PathBuf) -> anyhow::Result<()> {
    // read file
    let mut file = fs::File::open(&file_path).context("open file")?;
    let mut content = Vec::new();
    let size = file.read_to_end(&mut content).context("read file")?;

    // construct object header
    let header = format!("blob {size}\0");
    let header = header.as_bytes();

    // compute hash
    let mut hasher = Sha1::new();
    hasher.update(header);
    hasher.update(&content);
    let hash = hasher.finalize();
    let hash = hex::encode(hash);

    if write {
        // create object file
        fs::create_dir_all(format!(".git/objects/{}", &hash[..2]))
            .context("create directory in .git/objects")?;
        let file = fs::File::create(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
            .context("open object in .git/objects/")?;

        // write compressed content to file
        let mut writer = ZlibEncoder::new(file, Compression::default());
        writer.write_all(header).context("write header to file")?;
        writer
            .write_all(&content)
            .context("write object content to file")?;
    }

    println!("{}", hash);

    Ok(())
}
