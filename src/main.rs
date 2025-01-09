use anyhow::{bail, ensure, Context};

use clap::{Parser, Subcommand};
use flate2::write::ZlibEncoder;
use flate2::{read::ZlibDecoder, Compression};

use sha1::{Digest, Sha1};
use std::io::Write;
use std::{
    ffi::CStr,
    fs,
    io::{self, BufRead, BufReader, Read},
    path::PathBuf,
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

    HashObject {
        #[clap(short = 'w')]
        write: bool,
        file_path: PathBuf,
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

            let size = size
                .parse::<usize>()
                .with_context(|| format!(".git/objects file header has invalide size `{size}`"))?;

            // print object contents
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
            };
        }
        Command::HashObject { write, file_path } => {
            // read file
            let mut file = fs::File::open(&file_path).context("open file")?;
            let mut content = Vec::new();
            let size = file.read_to_end(&mut content).context("read file")?;

            // construct object header
            let header = format!("blob {size}\0");
            let header = header.as_bytes();

            // compute hash
            let mut hasher = Sha1::new();
            hasher.update(header);
            hasher.update(&content);
            let hash = hasher.finalize();
            let hash = hex::encode(hash);

            if write {
                // create object file
                fs::create_dir_all(format!(".git/objects/{}", &hash[..2]))
                    .context("create directory in .git/objects")?;
                let file = fs::File::create(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
                    .context("open object in .git/objects/")?;

                // write compressed content to file
                let mut writer = ZlibEncoder::new(file, Compression::default());
                let _ = writer.write_all(header);
                let _ = writer.write_all(&content);
            }

            println!("{}", hash);
        }
    }
    Ok(())
}

struct LimitReader<R: io::Read> {
    reader: R,
    limit: usize,
}

impl<R: io::Read> LimitReader<R> {
    fn new(reader: R, limit: usize) -> Self {
        Self { reader, limit }
    }
}

impl<R: io::Read> io::Read for LimitReader<R> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        if buf.len() > self.limit {
            buf = &mut buf[..self.limit + 1];
        }

        let n = self.reader.read(buf)?;
        if n > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "LimitReader read more than sepcified limit of {} bytes (read {} bytes)",
                    self.limit, n
                ),
            ));
        }

        self.limit -= n;
        Ok(n)
    }
}
