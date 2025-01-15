use anyhow::{bail, Context};
use std::{
    ffi::CStr,
    io::{self, BufRead, Read, Write},
};

use crate::objects::{Object, ObjectKind, ObjectMode, ObjectReader};

pub(crate) struct TreeEntry {
    mode: ObjectMode,
    name: String,
    sha: [u8; 20],
}

impl TreeEntry {
    fn new(mode: ObjectMode, name: &str, sha: [u8; 20]) -> Self {
        Self {
            mode,
            name: name.to_string(),
            sha,
        }
    }

    pub(crate) fn display<W: Write>(&self, writer: &mut W, name_only: bool) -> anyhow::Result<()> {
        if name_only {
            writeln!(writer, "{}", self.name).context("write tree entry")?;
        } else {
            let mode = self.mode.to_number();
            let object_type = self.mode.to_object_type();
            let hash = hex::encode(self.sha);
            let name = &self.name;
            writeln!(writer, "{mode:06} {object_type} {hash}\t{name}")
                .context("write tree entry")?;
        }

        Ok(())
    }

    pub(crate) fn parse_body(body_reader: &mut ObjectReader) -> anyhow::Result<(Self, usize)> {
        let mut buf = Vec::new();
        let n = body_reader
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
        body_reader
            .read_exact(&mut buf)
            .context("SHA is less than 20 bytes")?;

        let tree_entry = Self::new(mode, name, buf);

        Ok((tree_entry, n + 21))
    }
}

pub(crate) fn invoke(name_only: bool, object_hash: &str) -> anyhow::Result<()> {
    let mut object = Object::from(object_hash);

    let ObjectKind::Tree = object.kind else {
        bail!("provided objects is not a tree");
    };

    let mut stdout = io::stdout().lock();
    let mut n = 0;
    while n < object.expected_size {
        let (tree_entry, len) = TreeEntry::parse_body(&mut object.body_reader)?;
        n += len;
        tree_entry.display(&mut stdout, name_only)?;
    }

    Ok(())
}
