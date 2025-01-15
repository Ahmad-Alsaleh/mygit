use anyhow::{bail, Context};
use flate2::read::ZlibDecoder;
use std::{
    ffi::CStr,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

use crate::commands::ls_tree::TreeEntry;

pub(crate) enum ObjectKind {
    Blob,
    Tree,
}

pub(crate) enum ObjectMode {
    RegularFile,
    ExcutableFile,
    SymbolicLink,
    Directory,
}

impl ObjectMode {
    pub(crate) fn from_number(input: u32) -> Option<ObjectMode> {
        match input {
            100644 => Some(ObjectMode::RegularFile),
            100755 => Some(ObjectMode::ExcutableFile),
            120000 => Some(ObjectMode::SymbolicLink),
            40000 => Some(ObjectMode::Directory),
            _ => None,
        }
    }

    pub(crate) fn to_number(&self) -> u32 {
        match self {
            ObjectMode::RegularFile => 100644,
            ObjectMode::ExcutableFile => 100755,
            ObjectMode::SymbolicLink => 120000,
            ObjectMode::Directory => 40000,
        }
    }

    pub(crate) fn to_object_type(&self) -> &str {
        match self {
            Self::Directory => "tree",
            Self::RegularFile | Self::ExcutableFile | Self::SymbolicLink => "blob",
        }
    }
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

pub(crate) fn parse_object_header<R: BufRead>(
    reader: &mut R,
) -> anyhow::Result<(ObjectKind, usize)> {
    // read object file header
    let mut buf = Vec::new();
    reader
        .read_until(0, &mut buf)
        .context("read header from .git/objects")?;
    let header = CStr::from_bytes_with_nul(&buf)
        .expect("know there is exactly one null byte and it is at the end");
    let header = header
        .to_str()
        .context(".git/objects file header is invalid UTF-8")?;

    // parse object file header
    let Some((kind, size)) = header.split_once(' ') else {
        bail!(".git/objects file header doesn't have a space");
    };

    let kind = match kind {
        "blob" => ObjectKind::Blob,
        "tree" => ObjectKind::Tree,
        _ => bail!("object kind `{kind}` is not supported yet"),
    };

    let size = size
        .parse::<usize>()
        .with_context(|| format!(".git/objects file header has invalide size `{size}`"))?;

    Ok((kind, size))
}

pub(crate) fn parse_tree_object_body(
    reader: &mut ObjectReader,
) -> anyhow::Result<(TreeEntry, usize)> {
    let mut buf = Vec::new();
    let n = reader
        .read_until(0, &mut buf)
        .context("read entry in tree object")?;
    let tree_entry = CStr::from_bytes_with_nul(&buf)
        .expect("know there is exactly one null and it is at the end");
    let tree_entry = tree_entry
        .to_str()
        .context(".git/objects file header is invalid UTF-8")?;

    let Some((mode, name)) = tree_entry.split_once(' ') else {
        bail!("tree entry doesn't have a space");
    };

    let mode = mode
        .parse::<u32>()
        .with_context(|| format!("tree entry has invalid object mode `{mode}`"))?;
    let mode = ObjectMode::from_number(mode)
        .with_context(|| format!("tree entry has invalid object mode `{mode}`"))?;

    let mut buf = [0; 20];
    reader
        .read_exact(&mut buf)
        .context("SHA is less than 20 bytes")?;

    let tree_entry = TreeEntry::new(mode, name, buf);

    Ok((tree_entry, n + 21))
}
