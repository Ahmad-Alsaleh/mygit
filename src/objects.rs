use anyhow::{bail, Context};
use flate2::read::ZlibDecoder;
use flate2::{write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};
use std::{
    ffi::CStr,
    fmt::Display,
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

pub(crate) enum ObjectKind {
    Blob,
    Tree,
}

impl Display for ObjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectKind::Blob => write!(f, "blob"),
            ObjectKind::Tree => write!(f, "tree"),
        }
    }
}

pub(crate) enum ObjectMode {
    RegularFile,
    ExcutableFile,
    SymbolicLink,
    Directory,
}

impl ObjectMode {
    pub(crate) fn from_number(input: u32) -> anyhow::Result<ObjectMode> {
        match input {
            100644 => Ok(ObjectMode::RegularFile),
            100755 => Ok(ObjectMode::ExcutableFile),
            120000 => Ok(ObjectMode::SymbolicLink),
            40000 => Ok(ObjectMode::Directory),
            _ => bail!("Invalid object mode {input}"),
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

impl Object {
    pub(crate) fn open(object_hash: &str) -> Self {
        let object_path = Self::get_path(object_hash);
        let mut reader = Self::get_reader(&object_path).unwrap();
        let (kind, expected_size) = Self::parse_header(&mut reader).unwrap();

        Self {
            kind,
            expected_size,
            body_reader: reader,
        }
    }

    pub(crate) fn compute_hash(content: &[u8], size: usize) -> anyhow::Result<Vec<u8>> {
        let mut hasher = Sha1::new();
        hasher.update(format!("blob {size}\0"));
        hasher.update(content);
        let hash = hasher.finalize();
        let hash = hash.to_vec();

        Ok(hash)
    }

    pub(crate) fn write(hash: &str, content: &[u8], size: usize) -> anyhow::Result<()> {
        // create object file
        let object_path = Object::get_path(hash);
        fs::create_dir_all(
            object_path
                .parent()
                .expect("object path has at least one parent"),
        )
        .context("failed to create directory in .git/objects")?;
        let file = fs::File::create(object_path).context("open object in .git/objects/")?;

        // write compressed content to object file
        let header = format!("blob {size}\0");
        let header = header.as_bytes();
        let mut writer = ZlibEncoder::new(file, Compression::default());
        writer.write_all(header).context("write header to file")?;
        writer
            .write_all(content)
            .context("write object content to file")?;

        Ok(())
    }
}

impl Object {
    fn get_path(hash: &str) -> PathBuf {
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
