use crate::document::Document;
use crate::movement::{Direction, Movement, move_horizontally, move_vertically};
use crate::selection::Selection;
use anyhow::Result;
use std::path::Path;

pub struct Editor {
    pub document: Document,
    pub selection: Selection,
}

impl Editor {
    pub fn new(path: &Path) -> Result<Self> {
        let selection = Selection::point(0);
        let document = Document::open(path, None)?;

        Ok(Self {
            document: document,
            selection,
        })
    }

    pub fn move_h(&mut self, dir: Direction, count: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_horizontally(&rope, r, dir, count, Movement::Move));
    }

    pub fn move_v(&mut self, dir: Direction, count: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_vertically(&rope, r, dir, count, Movement::Move));
    }
}
