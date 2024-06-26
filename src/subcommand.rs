use anyhow::Context;
use chrono::Local;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::{self, BufRead, Read};
use std::io::{BufReader, BufWriter, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use std::{env, fs};

use crate::object::{BlobObject, TreeObject};

const TEMPORARY: &str = "temporary";

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
    let mut bufreader = BufReader::new(ZlibDecoder::new(BufReader::new(object)));
    let blob = BlobObject::read(&mut bufreader)?;
    print!("{}", blob.content);

    Ok(())
}

pub fn ls_tree(hash: &str) -> anyhow::Result<()> {
    let object = File::open(object_path_from_hash(hash))
        .with_context(|| anyhow::anyhow!("cannot open hash object file: {hash}"))?;
    let mut bufreader = BufReader::new(ZlibDecoder::new(BufReader::new(object)));
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

struct ObjectSize(usize);

impl ObjectSize {
    fn try_parse(reader: &mut dyn Read) -> anyhow::Result<ObjectSize> {
        let mut size = 0usize;
        let mut bitcount = 0usize;

        loop {
            let mut v = [0u8; 1];
            reader.read_exact(&mut v)?;
            let tmp = (v[0] & 0b0111_1111) as usize;
            size |= tmp << bitcount;
            bitcount += 7;

            if v[0] >> 7 == 0 {
                break;
            }
        }

        Ok(ObjectSize(size))
    }
}

struct ObjectSizeType {
    size: ObjectSize,
    object_type: u8,
}

impl ObjectSizeType {
    fn try_parse(reader: &mut dyn Read) -> anyhow::Result<ObjectSizeType> {
        let mut size = ObjectSize::try_parse(reader)?;
        let object_type = ((size.0 >> 4) & 0b111) as u8;

        // we need to preserve lowest 4 bits before we remove bits 5,6 and 7 by shifting
        let lower = size.0 & 0b1111;
        size.0 >>= 7;
        size.0 <<= 4;
        size.0 += lower;

        Ok(ObjectSizeType { size, object_type })
    }
}

#[derive(Debug)]
enum CopyCommand {
    FromReference { offset: usize, size: usize },
    Direct { data: Vec<u8> },
}

impl CopyCommand {
    fn try_parse(reader: &mut dyn Read) -> anyhow::Result<CopyCommand> {
        let mut header = [0u8; 1];
        reader.read_exact(&mut header)?;
        let header = header[0];

        match header >> 7 {
            0 => {
                let size = header & 0b0111_1111;
                let mut data = vec![0u8; size as usize];
                reader.read_exact(&mut data)?;
                Ok(CopyCommand::Direct { data })
            }
            1 => {
                let mut buffer = [0u8; 1];
                let mut offset = 0;
                let mut size = 0;

                for i in 0..4 {
                    if header & (1u8 << i) != 0 {
                        reader.read_exact(&mut buffer)?;
                        offset += (buffer[0] as usize) << (8 * i);
                    }
                }

                for i in 0..3 {
                    if header & (1u8 << (i + 4)) != 0 {
                        reader.read_exact(&mut buffer)?;
                        size += (buffer[0] as usize) << (8 * i);
                    }
                }

                if size == 0 {
                    size = 0x10000;
                }

                Ok(CopyCommand::FromReference { offset, size })
            }
            _ => unreachable!(),
        }
    }

    fn size(&self) -> usize {
        match self {
            CopyCommand::Direct { data } => data.len(),
            CopyCommand::FromReference { offset: _, size } => *size,
        }
    }
}

fn move_based_on_hash() -> anyhow::Result<String> {
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 1024];
    let mut f = File::open(TEMPORARY)?;
    loop {
        let n = f.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash: String = hex::encode(hasher.finalize()).to_string();
    let tree_object_path: PathBuf = object_path_from_hash(&hash).into();
    if let Some(folder) = tree_object_path.parent() {
        if !folder.exists() {
            fs::create_dir(folder)?;
        }
    }

    let mut f = File::open(TEMPORARY)?;
    let mut encoder = ZlibEncoder::new(File::create(tree_object_path)?, Compression::fast());
    io::copy(&mut f, &mut encoder)?;

    fs::remove_file(TEMPORARY)?;
    Ok(hash)
}

pub fn clone(url: &str, path: &Path, verbose: bool) -> anyhow::Result<()> {
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
    println!("version: {version} objects:{objects}");

    fs::create_dir(path)?;
    env::set_current_dir(path)?;
    init()?;

    let mut reader = BufReader::new(res);
    for _ in 0..objects {
        let ObjectSizeType { size, object_type } = ObjectSizeType::try_parse(&mut reader)?;
        let size = size.0;

        match object_type {
            1..=4 => {
                let mut zlib_reader = ZlibDecoder::new(reader);
                let mut content = Vec::new();
                zlib_reader.read_to_end(&mut content)?;
                let object_type = match object_type {
                    1 => "commit",
                    2 => "tree",
                    3 => "blob",
                    _ => todo!(),
                };

                let mut f = File::create(TEMPORARY)?;
                f.write_all(format!("{object_type} {size}\0").as_bytes())?;
                f.write_all(&content)?;
                drop(f);
                assert_eq!(size, content.len());

                let hash = move_based_on_hash()?;
                if verbose {
                    println!("{hash} {object_type} {size}");
                }
                reader = zlib_reader.into_inner();
            }
            7 => {
                let mut base_hash = [0u8; 20];
                reader.read_exact(&mut base_hash)?;
                let base_hash = hex::encode(base_hash).to_string();

                let mut f = File::create(TEMPORARY)?;

                // TODO: read sizes
                let mut zlib_reader = ZlibDecoder::new(reader);
                let _ = ObjectSize::try_parse(&mut zlib_reader)?.0;
                let final_size = ObjectSize::try_parse(&mut zlib_reader)?.0;
                // println!("base_hash: {}, final_size: {final_size}", object_path_from_hash(&base_hash));

                let mut current_size = 0;
                while current_size != final_size {
                    let copy_command = CopyCommand::try_parse(&mut zlib_reader)?;
                    match copy_command {
                        CopyCommand::FromReference { offset, size } => {
                            let mut decoder = BufReader::new(ZlibDecoder::new(BufReader::new(
                                File::open(object_path_from_hash(&base_hash))?,
                            )));
                            let mut content = Vec::new();
                            decoder.read_until(b'\0', &mut content)?;
                            let object_type =
                                &content[0..content.iter().position(|&x| x == b' ').unwrap()];
                            if current_size == 0 {
                                // write header
                                f.write_all(object_type)?;
                                f.write_all(b" ")?;
                                f.write_all(final_size.to_string().as_bytes())?;
                                f.write_all(b"\0")?;
                            }
                            // TODO
                            let mut content = Vec::new();
                            decoder.read_to_end(&mut content)?;
                            f.write_all(&content[offset..offset + size])?;
                        }
                        CopyCommand::Direct { ref data } => {
                            f.write_all(data)?;
                        }
                    }
                    // println!("  {copy_command:?}");
                    current_size += copy_command.size();
                }

                drop(f);
                let hash = move_based_on_hash()?;
                if verbose {
                    println!("{hash} {object_type} {size}");
                }
                reader = zlib_reader.into_inner();
            }
            _ => unimplemented!(),
        }
    }

    // TODO: implement git checkout (extract-tree)
    Command::new("git").arg("checkout").arg(head).output()?;

    Ok(())
}
