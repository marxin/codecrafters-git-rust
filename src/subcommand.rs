use anyhow::Context;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use crate::object::BlobObject;

fn object_path_from_hash(hash: &str) -> String {
    format!(".git/objects/{}/{}", &hash[0..2], &hash[2..])
}

pub fn init() -> anyhow::Result<()> {
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    File::create_new(".git/HEAD")?.write_all(b"ref: refs/heads/main\n")?;

    Ok(())
}

pub fn cat_file(hash: &str) -> anyhow::Result<()> {
    let object = File::open(object_path_from_hash(hash))
        .with_context(|| anyhow::anyhow!("cannot open hash object file: {hash}"))?;
    let decoder = ZlibDecoder::new(object);
    let mut bufreader = BufReader::new(decoder);
    let blob = BlobObject::read(&mut bufreader)?;
    print!("{}", blob.content);

    Ok(())
}

pub fn hash_object(path: &PathBuf, write: bool) -> anyhow::Result<String> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let mut reader = BufReader::new(file);

    let mut hasher = Sha1::new();
    let header = format!("blob {}\0", metadata.len());
    hasher.update(&header);
    let mut buffer = [0u8; 4096];
    loop {
        let n = reader.read(&mut buffer)?;
        hasher.update(&buffer[..n]);
        if n == 0 {
            break;
        }
    }

    let hash = hex::encode(hasher.finalize()).to_string();
    if write {
        let mut blob_file: File = File::open(path)?;
        let blob_object_path = PathBuf::from(object_path_from_hash(&hash));
        if let Some(folder) = blob_object_path.parent() {
            fs::create_dir(folder)?;
        }
        let object_file = BufWriter::new(File::create(blob_object_path)?);
        let mut encoder = ZlibEncoder::new(object_file, Compression::fast());

        let _ = encoder.write(header.as_bytes());
        loop {
            let n = blob_file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            let _ = encoder.write(&buffer[..n]);
        }
    }

    Ok(hash)
}