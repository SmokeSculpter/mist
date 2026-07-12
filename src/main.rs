mod chars;
mod command;
mod config;
mod document;
mod editor;
mod grapheme;
mod keymap;
mod line_ending;
mod mode;
mod movement;
mod search;
mod selection;
mod theme;
mod transaction;
mod ui;
use floem::prelude::*;
use std::path::Path;

use command::Context;
use config::load_config;
use editor::Editor;

/// Entry point: open the file named on the command line, wrap the `Editor` in a
/// reactive signal, and hand it to the root view. `RwSignal<Editor>` is the single
/// source of truth — the view reads it to render, key events update it in place.
fn main() -> anyhow::Result<()> {
    floem::launch(|| {
        let path = std::env::args().nth(1).expect("Usage: mist <file>");
        let config = load_config().expect("Should load fine");
        let editor = Editor::new(Path::new(&path)).expect("Failed to open");
        let context = RwSignal::new(Context::new(editor));
        ui::editor_view::editor_view(context)
    });

    Ok(())
}
