mod document;
mod editor;
mod mode;
mod ui;
use floem::prelude::*;
use std::path::Path;

use document::Document;

fn main() -> anyhow::Result<()> {
    floem::launch(|| {
        let path = std::env::args().nth(1).expect("Usage: mist <file>");
        let document =
            RwSignal::new(Document::open(Path::new(&path), None).expect("Failed to open"));
        ui::editor_view::editor_view(document)
    });

    Ok(())
}
