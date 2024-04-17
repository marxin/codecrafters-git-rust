use clap::{Args, Parser, Subcommand};
use std::str;

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
}

#[derive(Args)]
struct CatFileArgs {
    /// Pretty-print
    #[arg(short)]
    pretty: bool,

    /// Hash
    hash: String,
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
        None => todo!(),
    }
}
