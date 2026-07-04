mod chars;
mod document;
mod editor;
mod grapheme;
mod keymap;
mod line_ending;
mod mode;
mod movement;
mod selection;
mod ui;
use floem::prelude::*;
use std::path::Path;

use editor::Editor;

fn main() -> anyhow::Result<()> {
    floem::launch(|| {
        let path = std::env::args().nth(1).expect("Usage: mist <file>");
        let editor = RwSignal::new(Editor::new(Path::new(&path)).expect("Failed to open"));
        ui::editor_view::editor_view(editor)
    });

    Ok(())
}
