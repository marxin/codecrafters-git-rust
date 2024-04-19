use std::io::BufRead;

use itertools::Itertools;

pub type ShaHash = [u8; 20];

pub struct BlobObject {
    pub _size: usize,
    pub content: String,
}

impl BlobObject {
    pub fn read(input: &mut impl BufRead) -> anyhow::Result<Self> {
        let mut prefix = [0u8; 5];
        let _ = input.read_exact(&mut prefix);
        if &prefix != b"blob " {
            anyhow::bail!("Unexpected blob object start");
        }

        let mut size = Vec::new();
        input.read_until(b'\0', &mut size)?;
        size.pop();

        let mut content = String::new();
        input.read_to_string(&mut content)?;

        let size = String::from_utf8(size)?.parse::<usize>()?;
        if content.len() != size {
            anyhow::bail!(format!(
                "Blob content size {size}: does not match the actual content: {}",
                content.len()
            ))
        }
        Ok(Self {
            _size: size,
            content,
        })
    }
}

pub struct TreeItem {
    pub _mode: String,
    pub name: String,
    pub _hash: ShaHash,
}

pub struct TreeObject {
    pub _size: usize,
    pub items: Vec<TreeItem>,
}

impl TreeObject {
    pub fn read(input: &mut impl BufRead) -> anyhow::Result<Self> {
        let mut prefix = [0u8; 5];
        let _ = input.read_exact(&mut prefix);
        if &prefix != b"tree " {
            anyhow::bail!("Unexpected blob object start: {:?}", prefix);
        }

        let mut size = Vec::new();
        input.read_until(b'\0', &mut size)?;
        size.pop();
        let size = String::from_utf8(size)?.parse::<usize>()?;

        let mut items = Vec::new();
        loop {
            let mut line = Vec::new();
            let n = input.read_until(b'\0', &mut line)?;
            if n == 0 {
                break;
            }

            line.pop();
            let line = String::from_utf8(line)?;
            let parts = line.split(' ').collect_vec();
            let mut hash = ShaHash::default();
            input.read_exact(&mut hash)?;
            items.push(TreeItem {
                _mode: parts[0].to_owned(),
                name: parts[1].to_owned(),
                _hash: hash,
            });
        }

        Ok(Self { _size: size, items })
    }
}
