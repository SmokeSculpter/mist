use crate::document::Document;
use crate::mode::Mode;
use crate::movement::{
    Direction, Movement, move_horizontally, move_next_long_word_end, move_next_long_word_start,
    move_next_word_end, move_next_word_start, move_prev_long_word_end, move_prev_long_word_start,
    move_prev_word_end, move_prev_word_start, move_vertically,
};
use crate::selection::Selection;
use anyhow::Result;
use std::path::Path;

pub struct Editor {
    pub document: Document,
    pub selection: Selection,
    pub mode: Mode,
}

impl Editor {
    pub fn new(path: &Path) -> Result<Self> {
        let selection = Selection::point(0);
        let document = Document::open(path, None)?;

        Ok(Self {
            document: document,
            selection,
            mode: Mode::Normal,
        })
    }

    pub fn enter_insert(&mut self) {
        self.mode = Mode::Insert;
    }

    pub fn enter_normal(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn enter_select(&mut self) {
        self.mode = Mode::Select;
    }

    pub fn move_next_word_end(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_next_word_end(rope, r, 1));
    }

    pub fn move_next_long_word_end(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_next_long_word_end(rope, r, 1));
    }

    pub fn move_next_word_start(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_next_word_start(rope, r, 1));
    }

    pub fn move_next_long_word_start(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_next_long_word_start(rope, r, 1));
    }

    pub fn move_prev_word_end(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_prev_word_end(rope, r, 1));
    }

    pub fn move_prev_long_word_end(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_prev_long_word_end(rope, r, 1));
    }

    pub fn move_prev_word_start(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_prev_word_start(rope, r, 1));
    }

    pub fn move_prev_long_word_start(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_prev_long_word_start(rope, r, 1));
    }

    pub fn move_h(&mut self, dir: Direction, count: usize, movement: Movement) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_horizontally(rope, r, dir, count, movement));
    }

    pub fn move_v(&mut self, dir: Direction, count: usize, movement: Movement) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_vertically(rope, r, dir, count, movement));
    }
}
