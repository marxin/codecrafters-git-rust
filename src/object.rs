use std::io::BufRead;

pub struct BlobObject {
    pub size: usize,
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
        Ok(Self { size, content })
    }
}
