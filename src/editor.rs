//! The editor state hub (Helix's `Editor`/`View` role). Owns the document, the
//! selection, and the current mode; held in an `RwSignal<Editor>` so the view renders
//! from it and key events mutate it. Every motion method clones the selection,
//! transforms each range, and stores the result. Transactions/history land here later.

use crate::document::Document;
use crate::mode::Mode;
use crate::movement::{
    Direction, Movement, move_horizontally, move_next_long_word_end, move_next_long_word_start,
    move_next_word_end, move_next_word_start, move_prev_long_word_end, move_prev_long_word_start,
    move_prev_word_end, move_prev_word_start, move_vertically,
};
use crate::selection::{Range, Selection};
use anyhow::Result;
use ropey::RopeSlice;
use std::path::Path;

/// The single source of truth for editor state.
pub struct Editor {
    pub document: Document,
    pub selection: Selection,
    pub mode: Mode,
    pub count: Option<usize>,
}

impl Editor {
    pub fn new(path: &Path) -> Result<Self> {
        let selection = Selection::point(0);
        let document = Document::open(path, None)?;

        Ok(Self {
            document: document,
            selection,
            mode: Mode::Normal,
            count: None,
        })
    }

    /// Append another digit to count
    /// So pressing 1 then 2 should result in 12
    pub fn push_count_digit(&mut self, n: usize) {
        self.count = Some(self.count.unwrap_or(0) * 10 + n);
    }

    /// Take the count from current editor or default to 1
    /// Resets count to None
    pub fn take_count(&mut self) -> usize {
        self.count.take().unwrap_or(1)
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

    /// Select-mode counterpart to the `move_*_word_*` methods: run the same word
    /// motion `f` to find its target, but instead of replacing the range, extend the
    /// original one's head to the motion's cursor position (anchor preserved). Mirrors
    /// Helix `extend_word_impl` — the extend variants wrap the motion rather than
    /// threading a `Movement` flag through it (word motions define the whole range).
    fn extend_word(&mut self, f: impl Fn(RopeSlice, Range, usize) -> Range, n: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self.selection.clone().transform(|r| {
            let w = f(rope, r, n);
            r.put_cursor(rope, w.cursor(rope), true)
        });
    }

    pub fn extend_next_word_end(&mut self, n: usize) {
        self.extend_word(move_next_word_end, n);
    }

    pub fn move_next_word_end(&mut self, n: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_next_word_end(rope, r, n));
    }

    pub fn extend_next_long_word_end(&mut self, n: usize) {
        self.extend_word(move_next_long_word_end, n);
    }

    pub fn move_next_long_word_end(&mut self, n: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_next_long_word_end(rope, r, n));
    }

    pub fn extend_next_word_start(&mut self, n: usize) {
        self.extend_word(move_next_word_start, n);
    }

    pub fn move_next_word_start(&mut self, n: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_next_word_start(rope, r, n));
    }

    pub fn extend_next_long_word_start(&mut self, n: usize) {
        self.extend_word(move_next_long_word_start, n);
    }

    pub fn move_next_long_word_start(&mut self, n: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_next_long_word_start(rope, r, n));
    }

    pub fn extend_prev_word_end(&mut self, n: usize) {
        self.extend_word(move_prev_word_end, n);
    }

    pub fn move_prev_word_end(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_prev_word_end(rope, r, 1));
    }

    pub fn extend_prev_long_word_end(&mut self, n: usize) {
        self.extend_word(move_prev_long_word_end, n);
    }

    pub fn move_prev_long_word_end(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_prev_long_word_end(rope, r, 1));
    }

    pub fn extend_prev_word_start(&mut self, n: usize) {
        self.extend_word(move_prev_word_start, n);
    }

    pub fn move_prev_word_start(&mut self, n: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_prev_word_start(rope, r, n));
    }

    pub fn extend_prev_long_word_start(&mut self, n: usize) {
        self.extend_word(move_prev_long_word_start, n);
    }

    pub fn move_prev_long_word_start(&mut self, n: usize) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_prev_long_word_start(rope, r, n));
    }

    /// Horizontal motion (h/l). `movement` = Move in Normal, Extend in Select.
    pub fn move_h(&mut self, dir: Direction, count: usize, movement: Movement) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_horizontally(rope, r, dir, count, movement));
    }

    /// Vertical motion (j/k). Sticky column lives in `move_vertically`.
    pub fn move_v(&mut self, dir: Direction, count: usize, movement: Movement) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| move_vertically(rope, r, dir, count, movement));
    }
}
