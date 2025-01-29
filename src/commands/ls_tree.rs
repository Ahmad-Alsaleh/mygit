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
    entry_header_buf: Vec<u8>,
    sha_buf: [u8; 20],
}

impl TreeEntryIter {
    fn new(body_reader: ObjectReader, expected_size: usize) -> Self {
        Self {
            body_reader,
            remaining_bytes: expected_size,
            entry_header_buf: Vec::new(),
            sha_buf: [0; 20],
        }
    }
}

impl Iterator for TreeEntryIter {
    type Item = TreeEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_bytes == 0 {
            return None;
        }

        self.entry_header_buf.clear();
        let n = self
            .body_reader
            .read_until(0, &mut self.entry_header_buf)
            .expect("read entry in tree object");

        let (mode, name) = CStr::from_bytes_with_nul(&self.entry_header_buf)
            .expect("know there is exactly one null and it is at the end")
            .to_str()
            .expect(".git/objects file header is invalid UTF-8")
            .split_once(' ')
            .expect("tree entry doesn't have a space");

        let mode = mode
            .parse::<u32>()
            .expect("tree entry has invalid object mode");
        let mode = ObjectMode::from_number(mode).expect("tree entry has invalid object mode");

        self.body_reader
            .read_exact(&mut self.sha_buf)
            .expect("SHA is less than 20 bytes");

        self.remaining_bytes -= n + 20;

        Some(TreeEntry::new(mode, name, self.sha_buf))
    }
}

pub(crate) fn invoke(name_only: bool, object_hash: &str) -> anyhow::Result<()> {
    let object = Object::open(object_hash);

    let ObjectKind::Tree = object.kind else {
        bail!("provided objects is not a tree");
    };

    let mut stdout = io::stdout().lock();
    for tree_entry in TreeEntryIter::new(object.body_reader, object.expected_size) {
        tree_entry.display(&mut stdout, name_only)?;
    }

    Ok(())
}
