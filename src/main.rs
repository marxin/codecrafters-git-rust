use clap::{Args, Parser, Subcommand};
use std::{path::PathBuf, str};

mod object;
mod subcommand;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes a new git repository
    Init,
    /// Read a blob git object
    CatFile(CatFileArgs),
    /// Write a blob git object
    HashObject(HashObjectArgs),
}

#[derive(Args)]
struct CatFileArgs {
    /// Pretty-print
    #[arg(short)]
    pretty: bool,

    /// Hash
    hash: String,
}

#[derive(Args)]
struct HashObjectArgs {
    /// Path to a file
    path: PathBuf,

    /// Write to object storage
    #[arg(short)]
    write: bool,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            if let Err(err) = subcommand::init() {
                eprintln!("Git init failed with: {err}");
            }
        }
        Some(Commands::CatFile(CatFileArgs { pretty, hash })) => match (pretty, hash) {
            (false, _) => eprintln!("pretty-print command is expected for cat-file subcommand"),
            (true, hash) => {
                if let Err(err) = subcommand::cat_file(&hash) {
                    eprintln!("git cat-file failed with: {err}");
                }
            }
        },
        Some(Commands::HashObject(HashObjectArgs { path, write })) => {
            let hash = subcommand::hash_object(&path, write);
            if let Err(err) = hash {
                eprintln!("git hash-object failed with: {err}");
            } else {
                println!("{}", hash.unwrap());
            }
        }
        None => todo!(),
    }
}
