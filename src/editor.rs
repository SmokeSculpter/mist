//! The editor state hub (Helix's `Editor`/`View` role). Owns the document, the
//! selection, and the current mode; held in an `RwSignal<Editor>` so the view renders
//! from it and key events mutate it. Every motion method clones the selection,
//! transforms each range, and stores the result. Transactions/history land here later.

use crate::document::Document;
use crate::grapheme::{next_grapheme_boundary, prev_grapheme_boundary};
use crate::mode::Mode;
use crate::movement::{
    Direction, Movement, line_char_len, move_horizontally, move_next_long_word_end,
    move_next_long_word_start, move_next_word_end, move_next_word_start, move_prev_long_word_start,
    move_prev_word_start, move_vertically,
};
use crate::search::find_nth_char;
use crate::selection::{Range, Selection};
use anyhow::Result;
use ropey::RopeSlice;
use std::path::Path;

/// Waiting for the target char of an f/t/F/T motion. `Some` = the next Character
/// key is a literal target, not a command. Captures the params fixed at f-press time.
pub struct PendingFind {
    pub count: usize,
    pub dir: Direction,
    pub inclusive: bool,
    pub extend: bool,
}

/// The single source of truth for editor state.
pub struct Editor {
    pub document: Document,
    pub selection: Selection,
    pub mode: Mode,
    pub count: Option<usize>,
    pub pending_find: Option<PendingFind>,
    pub pending_goto: bool,
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
            pending_find: None,
            pending_goto: false,
        })
    }

    fn goto(&mut self, pos: usize, extend: bool) {
        let rope = self.document.rope().slice(..);
        self.selection = self
            .selection
            .clone()
            .transform(|r| r.put_cursor(rope, pos, extend))
    }

    pub fn goto_file_start(&mut self, extend: bool) {
        self.goto(0, extend);
    }

    pub fn goto_file_end(&mut self, extend: bool) {
        let rope = self.document.rope().slice(..);
        let pos = prev_grapheme_boundary(rope, rope.len_chars());
        self.goto(pos, extend);
    }

    pub fn goto_line_start(&mut self, extend: bool) {
        let rope = self.document.rope().slice(..);
        self.selection = self.selection.clone().transform(|r| {
            let line = rope.char_to_line(r.cursor(rope));
            r.put_cursor(rope, rope.line_to_char(line), extend)
        });
    }

    pub fn goto_line_end(&mut self, extend: bool) {
        let rope = self.document.rope().slice(..);
        self.selection = self.selection.clone().transform(|r| {
            let line = rope.char_to_line(r.cursor(rope));
            let start = rope.line_to_char(line);
            let end = start + line_char_len(rope, line);
            // Block caret sits on last char, not past line's last grapheme;
            // clamp to 'start' so an empty line stays put
            let pos = prev_grapheme_boundary(rope, end).max(start);
            r.put_cursor(rope, pos, extend)
        })
    }

    pub fn find_char(&mut self, find: &PendingFind, char: char) {
        let rope = self.document.rope().slice(..);
        self.selection = self.selection.clone().transform(|r| {
            let cursor_anchor = r.cursor(rope);
            let cursor_head = next_grapheme_boundary(rope, cursor_anchor);

            // Where to begin the search. 't' skips one extra char so a repeated 't'
            // doesn't get stuck on the char it's already sitting before.
            let search_start = match (find.inclusive, find.dir) {
                (true, Direction::Forward) => cursor_head,
                (true, Direction::Backward) => cursor_anchor,
                (false, Direction::Forward) => cursor_head + 1,
                (false, Direction::Backward) => cursor_anchor.saturating_sub(1),
            };

            match find_nth_char(find.count, rope, char, search_start, find.dir) {
                None => r,
                Some(found) => {
                    // 't' stops one short of the match (on the near side)
                    let pos = match (find.inclusive, find.dir) {
                        (true, _) => found,
                        (false, Direction::Forward) => found - 1,
                        (false, Direction::Backward) => found + 1,
                    };
                    if find.extend {
                        r.put_cursor(rope, pos, true)
                    } else {
                        Range::point(r.cursor(rope)).put_cursor(rope, pos, true)
                    }
                }
            }
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
