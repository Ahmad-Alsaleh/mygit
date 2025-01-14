use anyhow::{bail, ensure, Context};
use std::io;

use crate::utils::{
    limit_reader::LimitReader,
    object_utils::{get_object_path, get_object_reader, parse_object_header, ObjectKind},
};

pub(crate) fn invoke(pretty_print: bool, object_hash: &str) -> anyhow::Result<()> {
    ensure!(
        pretty_print,
        "Missing -p flag: Object type should be given using -p as object mode is not supported now"
    );

    let file_path = get_object_path(object_hash);
    let mut reader = get_object_reader(&file_path)?;
    let (kind, size) = parse_object_header(&mut reader)?;

    match kind {
        ObjectKind::Blob => {
            let mut reader = LimitReader::new(reader, size);
            let n = io::copy(&mut reader, &mut io::stdout().lock())
                .context("write contents of .git/object file to stdout")?;
            ensure!(
                n as usize == size,
                ".git/object file has extra trailing bytes, expected {size} bytes only"
            )
        }
        _ => bail!("only blobs are supported right now"),
    };

    Ok(())
}
