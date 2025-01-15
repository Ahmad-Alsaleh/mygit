use std::io::{self, Write};

use anyhow::{bail, Context};

use crate::utils::object_utils::{
    get_object_path, get_object_reader, parse_object_header, parse_tree_object_body, ObjectKind,
    ObjectMode,
};

pub(crate) struct TreeEntry {
    mode: ObjectMode,
    name: String,
    sha: [u8; 20],
}

impl TreeEntry {
    pub(crate) fn new(mode: ObjectMode, name: &str, sha: [u8; 20]) -> Self {
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

pub(crate) fn invoke(name_only: bool, object_hash: &str) -> anyhow::Result<()> {
    let file_path = get_object_path(object_hash);
    let mut reader = get_object_reader(&file_path)?;
    let (kind, size) = parse_object_header(&mut reader)?;

    match kind {
        ObjectKind::Tree => {
            let mut stdout = io::stdout().lock();
            let mut n = 0;
            while n < size {
                let (tree_entry, len) = parse_tree_object_body(&mut reader)?;
                n += len;
                tree_entry.display(&mut stdout, name_only)?;
            }
        }
        _ => bail!("provided objects is not a tree"),
    }

    Ok(())
}
