use std::{fs, io::Write};
use std::fs::File;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes a new git repository
    Init
}

fn init() -> anyhow::Result<()> {
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    File::create_new(".git/HEAD")?.write_all(b"ref: refs/heads/main\n")?;

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            if let Err(err) = init() {
                eprintln!("Git init failed with: {err}");
            }
        }
        None => todo!()
    }

}
