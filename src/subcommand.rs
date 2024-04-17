use anyhow::Context;
use flate2::read::ZlibDecoder;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Write};

use crate::object::BlobObject;

pub fn init() -> anyhow::Result<()> {
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    File::create_new(".git/HEAD")?.write_all(b"ref: refs/heads/main\n")?;

    Ok(())
}

pub fn cat_file(hash: &str) -> anyhow::Result<()> {
    let object = File::open(format!(".git/objects/{}/{}", &hash[0..2], &hash[2..]))
        .with_context(|| anyhow::anyhow!("cannot open hash object file: {hash}"))?;
    let decoder = ZlibDecoder::new(object);
    let mut bufreader = BufReader::new(decoder);
    let blob = BlobObject::read(&mut bufreader)?;
    print!("{}", blob.content);

    Ok(())
}
