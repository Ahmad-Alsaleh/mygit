use anyhow::{bail, ensure, Context};
use clap::{Parser, Subcommand};
use flate2::read::ZlibDecoder;
use std::{
    ffi::CStr,
    fs,
    io::{self, BufRead, BufReader, Read, Write},
};

/// Git in Rust
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create an empty Git repository
    Init,

    /// Provide contents or details of repository objects
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,
        object_hash: String,
    },
}

enum ObjectKind {
    Blob,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory");
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
            ensure!(
                pretty_print,
                "Missing -p flag: Object type should be given using -p as object mode is not supported now"
            );

            // construct file reader
            let file = fs::File::open(format!(
                ".git/objects/{}/{}",
                &object_hash[..2],
                &object_hash[2..]
            ))
            .context("open in .git/objects/")?;
            let decompressor = ZlibDecoder::new(file);
            let mut reader = BufReader::new(decompressor);

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

            let size = size.parse::<usize>().context(format!(
                ".git/objects file header has invalide size `{size}`"
            ))?;

            // read object contents
            buf.resize(size, 0);
            reader
                .read_exact(&mut buf)
                .context(".git/objects file contents exceeded given size in header")?;
            let n = reader
                .read(&mut [0])
                .context("validate EOF in .git/objects")?;
            ensure!(
                n == 0,
                ".git/object file has extra trailing bytes, expected {size} bytes only"
            );

            // print object contents
            let mut stdout = io::stdout().lock();
            match kind {
                ObjectKind::Blob => stdout.write_all(&buf).context("write contents to stdout")?,
            };
        }
    }
    Ok(())
}
