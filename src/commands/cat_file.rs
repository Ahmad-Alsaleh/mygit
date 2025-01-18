use anyhow::{bail, ensure, Context};
use std::io;

use crate::objects::{Object, ObjectKind};
use crate::utils::limit_reader::LimitReader;

pub(crate) fn invoke(pretty_print: bool, object_hash: &str) -> anyhow::Result<()> {
    ensure!(
        pretty_print,
        "Missing -p flag: Object type should be given using -p as object mode is not supported now"
    );

    let object = Object::new(object_hash);

    let ObjectKind::Blob = object.kind else {
        bail!("object type `{}` is not supported right now", object.kind);
    };

    let mut reader = LimitReader::new(object.body_reader, object.expected_size);
    let n = io::copy(&mut reader, &mut io::stdout().lock())
        .context("write contents of .git/object file to stdout")?;
    ensure!(
        n as usize == object.expected_size,
        ".git/object file has extra trailing bytes, expected {} bytes only",
        object.expected_size
    );

    Ok(())
}
