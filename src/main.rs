use clap::{Parser, Subcommand};
use std::{path::PathBuf, str};

mod object;
mod subcommand;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes a new git repository
    Init,
    /// Read a blob git object
    CatFile {
        /// Pretty-print
        #[arg(short)]
        pretty: bool,

        /// Hash
        hash: String,
    },
    /// Write a blob git object
    HashObject {
        /// Path to a file
        path: PathBuf,

        /// Write to object storage
        #[arg(short)]
        write: bool,
    },
    /// Inspect a tree object
    LsTree {
        /// Hash
        hash: String,

        /// Print only names
        #[arg(long)]
        name_only: bool,
    },
    /// Write tree object
    WriteTree,
    /// Commit
    CommitTree {
        /// Hash
        tree_object: String,

        /// Parent commit hash
        #[arg(short)]
        parent: String,

        /// Commit message
        #[arg(short)]
        message: String,
    },
    /// Clone a repository with HTTP protocol
    Clone {
        /// URL
        url: String,

        /// Output directory
        dir: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            if let Err(err) = subcommand::init() {
                eprintln!("Git init failed with: {err}");
            }
        }
        Commands::CatFile { pretty, hash } => match (pretty, hash) {
            (false, _) => eprintln!("--pretty-print option is expected for cat-file subcommand"),
            (true, hash) => {
                if let Err(err) = subcommand::cat_file(&hash) {
                    eprintln!("git cat-file failed with: {err}");
                }
            }
        },
        Commands::HashObject { path, write } => {
            let hash = subcommand::hash_object(&path, write);
            if let Err(err) = hash {
                eprintln!("git hash-object failed with: {err}");
            } else {
                println!("{}", hash.unwrap());
            }
        }
        Commands::LsTree { name_only, hash } => match (name_only, hash) {
            (false, _) => eprintln!("--name-only option is expected for ls-tree subcommand"),
            (true, hash) => {
                if let Err(err) = subcommand::ls_tree(&hash) {
                    eprintln!("git ls-tree failed with: {err}");
                }
            }
        },
        Commands::WriteTree => {
            let hash = subcommand::write_tree();
            if let Err(err) = hash {
                eprintln!("git write-tree failed with: {err}");
            } else {
                println!("{}", hash.unwrap());
            }
        }
        Commands::CommitTree {
            tree_object,
            parent,
            message,
        } => {
            let hash = subcommand::commit_tree(&tree_object, &parent, &message);
            if let Err(err) = hash {
                eprintln!("git commit-tree failed with: {err}");
            } else {
                println!("{}", hash.unwrap());
            }
        }
        Commands::Clone { url, dir } => {
            let hash = subcommand::clone(&url, &dir);
            if let Err(err) = hash {
                eprintln!("git clone failed with: {err}");
            }
        }
    }
}
