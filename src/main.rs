use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub(crate) mod commands;
pub(crate) mod objects;
pub(crate) mod utils;

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

    /// List the contents of a tree object
    LsTree {
        /// list only filenames, one per line.
        #[clap(long = "name-only")]
        name_only: bool,
        object_hash: String,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => {
            commands::init::invoke();
            println!("Initialized git directory");
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
            commands::cat_file::invoke(pretty_print, &object_hash)?;
        }
        Command::HashObject { write, file_path } => {
            commands::hash_object::invoke(write, file_path)?;
        }
        Command::LsTree {
            name_only,
            object_hash,
        } => {
            commands::ls_tree::invoke(name_only, &object_hash)?;
        }
    };

    Ok(())
}
