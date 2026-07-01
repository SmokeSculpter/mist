mod document;
use std::path::PathBuf;

use document::Document;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("usage: mist <file>"))?;

    let document = Document::open(&path, None)?;

    print!("{}", document.rope());

    Ok(())
}
