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

pub(crate) type ObjectReader = BufReader<ZlibDecoder<fs::File>>;

pub(crate) struct Object {
    pub(crate) kind: ObjectKind,
    pub(crate) expected_size: usize,
    pub(crate) body_reader: ObjectReader,
}

impl From<&str> for Object {
    fn from(object_hash: &str) -> Self {
        let object_path = Self::get_path(object_hash);
        let mut reader = Self::get_reader(&object_path).unwrap();
        let (kind, expected_size) = Self::parse_header(&mut reader).unwrap();

        Self {
            kind,
            expected_size,
            body_reader: reader,
        }
    }
}

impl Object {
    pub fn get_path(hash: &str) -> PathBuf {
        PathBuf::from(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
    }

    fn get_reader(file_path: &Path) -> anyhow::Result<ObjectReader> {
        let file = fs::File::open(file_path).context("open in .git/objects/")?;
        let decompressor = ZlibDecoder::new(file);
        let reader = BufReader::new(decompressor);

        Ok(reader)
    }

    fn parse_header(reader: &mut ObjectReader) -> anyhow::Result<(ObjectKind, usize)> {
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
}
