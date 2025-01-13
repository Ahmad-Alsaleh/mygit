use anyhow::{bail, Context};
use flate2::read::ZlibDecoder;
use std::{
    ffi::CStr,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

pub(crate) enum ObjectKind {
    Blob,
}

type ObjectReader = BufReader<ZlibDecoder<fs::File>>;

pub(crate) fn get_object_path(object_hash: &str) -> PathBuf {
    PathBuf::from(format!(
        ".git/objects/{}/{}",
        &object_hash[..2],
        &object_hash[2..]
    ))
}

pub(crate) fn get_object_reader(file_path: &Path) -> anyhow::Result<ObjectReader> {
    let file = fs::File::open(file_path).context("open in .git/objects/")?;
    let decompressor = ZlibDecoder::new(file);
    let reader = BufReader::new(decompressor);

    Ok(reader)
}

pub(crate) fn get_object_header<R: BufRead>(reader: &mut R) -> anyhow::Result<(ObjectKind, usize)> {
    // read object file header
    let mut buf = Vec::new();
    let _ = reader.read_until(0, &mut buf);
    let header = CStr::from_bytes_with_nul(&buf)
        .expect("know there is exactly one null byte and it is at the end");
    let header = header
        .to_str()
        .context(".git/objects file header is invalid UTF-8")?;

    // parse object file header
    let Some((kind, size)) = header.split_once(' ') else {
        bail!(".git/objects file header didn't have a space");
    };

    let kind = match kind {
        "blob" => ObjectKind::Blob,
        _ => bail!("object kind `{kind}` is not supported yet"),
    };

    let size = size
        .parse::<usize>()
        .with_context(|| format!(".git/objects file header has invalide size `{size}`"))?;

    Ok((kind, size))
}
