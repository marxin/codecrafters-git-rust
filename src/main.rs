use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str;
use std::{fs, io::Write};

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use flate2::read::ZlibDecoder;

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

fn init() -> anyhow::Result<()> {
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    File::create_new(".git/HEAD")?.write_all(b"ref: refs/heads/main\n")?;

    Ok(())
}

struct BlobObject {
    size: usize,
    content: String,
}

impl BlobObject {
    fn parse(input: &mut impl BufRead) -> anyhow::Result<Self> {
        let mut prefix = [0u8; 5];
        let _ = input.read_exact(&mut prefix);
        if &prefix != b"blob " {
            anyhow::bail!("Unexpected blob object start");
        }

        let mut size = Vec::new();
        input.read_until(b'\0', &mut size)?;

        let mut content = String::new();
        input.read_to_string(&mut content)?;

        let size = String::from_utf8(size)?.parse::<usize>()?;
        if content.len() != size {
            anyhow::bail!(
                "Blob content size {size}: does not match the actual content: {}",
                content.len()
            )
        }
        Ok(Self { size: 1, content })
    }
}

fn cat_file(hash: &str) -> anyhow::Result<()> {
    let object = File::open(format!(".git/objects/{}/{}", &hash[0..2], &hash[2..]))
        .with_context(|| anyhow::anyhow!("cannot open hash object file: {hash}"))?;
    let decoder = ZlibDecoder::new(object);
    let mut bufreader = BufReader::new(decoder);
    let blob = BlobObject::parse(&mut bufreader)?;
    if blob.content.len() != blob.size {
        anyhow::bail!("Blob content size does not match")
    }
    print!("{}", blob.content);

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
        Some(Commands::CatFile(CatFileArgs { pretty, hash })) => match (pretty, hash) {
            (false, _) => eprintln!("pretty-print command is expected for cat-file subcommand"),
            (true, hash) => {
                if let Err(err) = cat_file(&hash) {
                    eprintln!("Git cat-file failed with: {err}");
                }
            }
        },
        None => todo!(),
    }
}
