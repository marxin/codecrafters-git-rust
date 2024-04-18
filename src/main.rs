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
    /// Inspect a tree object
    LsTree(LsTreeArgs),
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

#[derive(Args)]
struct LsTreeArgs {
    /// Hash
    hash: String,

    /// Print only names
    #[arg(long)]
    name_only: bool,
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
            (false, _) => eprintln!("--pretty-print option is expected for cat-file subcommand"),
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
        Some(Commands::LsTree(LsTreeArgs { name_only, hash })) => match (name_only, hash) {
            (false, _) => eprintln!("--name-only option is expected for ls-tree subcommand"),
            (true, hash) => {
                if let Err(err) = subcommand::ls_tree(&hash) {
                    eprintln!("git ls-tree failed with: {err}");
                }
            }
        },
        None => todo!(),
    }
}
