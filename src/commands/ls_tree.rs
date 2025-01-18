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
}

struct TreeEntryIter {
    body_reader: ObjectReader,
    remaining_bytes: usize,
}

impl TreeEntryIter {
    fn new(body_reader: ObjectReader, expected_size: usize) -> Self {
        Self {
            body_reader,
            remaining_bytes: expected_size,
        }
    }
}

impl Iterator for TreeEntryIter {
    type Item = TreeEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_bytes == 0 {
            return None;
        }

        let mut buf = Vec::new();
        let n = self
            .body_reader
            .read_until(0, &mut buf)
            .expect("read entry in tree object");

        let tree_entry = CStr::from_bytes_with_nul(&buf)
            .expect("know there is exactly one null and it is at the end");
        let tree_entry = tree_entry
            .to_str()
            .expect(".git/objects file header is invalid UTF-8");

        let (mode, name) = tree_entry
            .split_once(' ')
            .expect("tree entry doesn't have a space");

        let mode = mode
            .parse::<u32>()
            .expect("tree entry has invalid object mode");
        let mode = ObjectMode::from_number(mode).expect("tree entry has invalid object mode");

        let mut buf = [0; 20];
        self.body_reader
            .read_exact(&mut buf)
            .expect("SHA is less than 20 bytes");

        self.remaining_bytes -= n + 20;

        Some(TreeEntry::new(mode, name, buf))
    }
}

pub(crate) fn invoke(name_only: bool, object_hash: &str) -> anyhow::Result<()> {
    let object = Object::new(object_hash);

    let ObjectKind::Tree = object.kind else {
        bail!("provided objects is not a tree");
    };

    let mut stdout = io::stdout().lock();
    for tree_entry in TreeEntryIter::new(object.body_reader, object.expected_size) {
        tree_entry.display(&mut stdout, name_only)?;
    }

    Ok(())
}
