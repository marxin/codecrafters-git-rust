use anyhow::Context;
use chrono::Local;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
use std::fs;
use std::fs::File;
use std::io::{self, Read};
use std::io::{BufReader, BufWriter, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::str;

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

        encoder.write_all(header.as_bytes())?;
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
            if entry.metadata()?.permissions().mode() & 0o011 != 0 {
                "100755"
            } else {
                "100644"
            }
        } else if entry.is_dir() {
            "40000"
        } else {
            todo!("unknown entry type")
        };

        content.extend(mode.as_bytes());
        content.extend(b" ");
        content.extend(filename.as_bytes());
        content.extend(b"\0");
        content.extend(&hash);
        assert_eq!(hash.len(), 20);
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
    encoder.write_all(header.as_bytes())?;
    encoder.write_all(&content)?;
    // TODO: check return values from write

    Ok(hash)
}

pub fn write_tree() -> anyhow::Result<String> {
    let cwd = Path::new(".");
    write_dir_hash(cwd)
}

pub fn commit_tree(tree: &str, parent: &str, message: &str) -> anyhow::Result<String> {
    let mut content = String::new();
    content.push_str(&format!("tree {tree}\n"));
    content.push_str(&format!("parent {parent}\n"));

    let now = Local::now();
    let tz = now.offset().to_string().replace(':', "");
    let author_line = format!(
        "Martin Liska <martin.liska@hey.com> {} {tz}",
        now.timestamp()
    );
    content.push_str(&format!("author {}\n", &author_line));
    content.push_str(&format!("commiter {}\n\n", &author_line));
    content.push_str(message);
    content.push('\n');

    let mut hasher = Sha1::new();
    let header = format!("commit {}\0", content.len());
    hasher.update(&header);
    hasher.update(&content);

    let hash = hex::encode(hasher.finalize()).to_string();

    let tree_object_path = PathBuf::from(object_path_from_hash(&hash));
    if let Some(folder) = tree_object_path.parent() {
        if !folder.exists() {
            fs::create_dir(folder)?;
        }
    }

    let commit_file = BufWriter::new(File::create(&tree_object_path)?);
    let mut encoder = ZlibEncoder::new(commit_file, Compression::fast());
    encoder.write_all(header.as_bytes())?;
    encoder.write_all(content.as_bytes())?;
    // TODO: check return values from write

    Ok(hash)
}

pub fn clone(url: &str, _path: &Path) -> anyhow::Result<()> {
    let body =
        reqwest::blocking::get(format!("{url}/info/refs?service=git-upload-pack"))?.text()?;
    let mut lines = body.lines();
    assert!(lines.next().is_some_and(|l| l.starts_with("001e")));
    let head: String = lines
        .next()
        .unwrap()
        .strip_prefix("0000")
        .unwrap()
        .chars()
        .skip(4)
        .take(40)
        .collect();
    println!("{head}");

    let client = reqwest::blocking::Client::new();
    let mut res = client
        .post(format!("{url}/git-upload-pack"))
        .body(format!("0032want {head}\n00000009done\n"))
        .send()?;

    println!("{}", res.status());
    const EXPECTED_PREFIX: &str = "0008NAK\nPACK";
    let mut prefix = [0u8; EXPECTED_PREFIX.len()];
    res.read_exact(&mut prefix)?;
    assert_eq!(&prefix, EXPECTED_PREFIX.as_bytes());
    let mut buffer = [0u8; 4];
    res.read_exact(&mut buffer)?;
    let version = u32::from_be_bytes(buffer);
    res.read_exact(&mut buffer)?;
    let objects = u32::from_be_bytes(buffer);
    println!("{version} {objects}");

    for _ in 0..objects {
        let mut v = [0u8; 1];
        res.read_exact(&mut v)?;
        println!("{:0b}", v[0]);
        assert!(v[0] & 0b1000_0000 != 0);

        // First 3 bits encode object type
        let object_type = (v[0] >> 4) & 0b111;
        let mut size = (v[0] & 0b1111) as usize;
        let mut bitcount = 4usize;

        loop {
            res.read_exact(&mut v)?;
            let tmp = (v[0] & 0b0111_1111) as usize;
            size |= tmp << bitcount;
            bitcount += 7;

            if v[0] >> 7 == 0 {
                break;
            }
        }

        println!("size={size}");
        let mut zlib_reader = ZlibDecoder::new(&mut res);
        let mut content = vec![0u8; size];
        zlib_reader.read_exact(&mut content)?;

        println!(
            "objet_type={object_type}, size={}, start={}",
            content.len(),
            str::from_utf8(&content[..32])?
        );
        println!("total_in={}", zlib_reader.total_in());
        // TODO
        todo!();
    }

    Ok(())
}
