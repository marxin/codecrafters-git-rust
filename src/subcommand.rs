use anyhow::Context;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::object::{BlobObject, TreeObject};

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
    let mut bufreader = BufReader::new(ZlibDecoder::new(object));
    let blob = BlobObject::read(&mut bufreader)?;
    print!("{}", blob.content);

    Ok(())
}

pub fn ls_tree(hash: &str) -> anyhow::Result<()> {
    let object = File::open(object_path_from_hash(hash))
        .with_context(|| anyhow::anyhow!("cannot open hash object file: {hash}"))?;
    let mut bufreader = BufReader::new(ZlibDecoder::new(object));
    let tree = TreeObject::read(&mut bufreader)?;
    for entry in tree.items {
        println!("{}", entry.name);
    }

    Ok(())
}

pub fn hash_object(path: &PathBuf, write: bool) -> anyhow::Result<String> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let mut reader = BufReader::new(file);

    let mut hasher = Sha1::new();
    let header = format!("blob {}\0", metadata.len());
    hasher.update(&header);
    io::copy(&mut reader, &mut hasher)?;

    let hash = hex::encode(hasher.finalize()).to_string();
    if write {
        let mut blob_file: File = File::open(path)?;
        let blob_object_path = PathBuf::from(object_path_from_hash(&hash));
        if let Some(folder) = blob_object_path.parent() {
            if !folder.exists() {
                fs::create_dir(folder)?;
            }
        }
        if blob_object_path.exists() {
            return Ok(hash);
        }
        let object_file = BufWriter::new(File::create(blob_object_path)?);
        let mut encoder = ZlibEncoder::new(object_file, Compression::fast());

        let _ = encoder.write(header.as_bytes());
        io::copy(&mut blob_file, &mut encoder)?;
    }

    Ok(hash)
}

fn write_dir_hash(path: &Path) -> anyhow::Result<String> {
    let mut entries = fs::read_dir(path)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort();

    let mut content = Vec::new();
    for entry in entries {
        let filename = entry
            .file_name()
            .ok_or(anyhow::anyhow!("missing filename"))?
            .to_str()
            .ok_or(anyhow::anyhow!("cannot get string"))?;
        if filename.starts_with('.') {
            continue;
        }

        let hash: String = if entry.is_file() {
            hash_object(&entry, true)
        } else {
            write_dir_hash(&entry)
        }?;
        let hash = hex::decode(hash)?;

        let mode = if entry.is_file() {
            if entry.metadata()?.permissions().mode() & 0o222 != 0 {
                "100755"
            } else {
                "100644"
            }
        } else if entry.is_dir() {
            "040000"
        } else {
            todo!("unknown entry type")
        };

        content.extend(mode.as_bytes());
        content.extend(b" ");
        content.extend(filename.as_bytes());
        content.extend(b"\0");
        content.extend(&hash);
    }

    let mut hasher = Sha1::new();
    let header = format!("tree {}\0", content.len());
    hasher.update(&header);
    hasher.update(&content);

    let hash = hex::encode(hasher.finalize()).to_string();

    let tree_object_path = PathBuf::from(object_path_from_hash(&hash));
    if let Some(folder) = tree_object_path.parent() {
        if !folder.exists() {
            fs::create_dir(folder)?;
        }
    }

    let tree_file = BufWriter::new(File::create(&tree_object_path)?);
    let mut encoder = ZlibEncoder::new(tree_file, Compression::fast());
    let _ = encoder.write(header.as_bytes());
    let _ = encoder.write(&content);
    // TODO: check return values from write

    Ok(hash)
}

pub fn write_tree() -> anyhow::Result<String> {
    let cwd = Path::new(".");
    write_dir_hash(cwd)
}
