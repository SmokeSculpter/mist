//! The editor state hub (Helix's `Editor`/`View` role). Owns the document, the
//! selection, and the current mode; held in an `RwSignal<Editor>` so the view renders
//! from it and key events mutate it. Every motion method clones the selection,
//! transforms each range, and stores the result. Transactions/history land here later.

use crate::document::Document;
use crate::grapheme::{next_grapheme_boundary, prev_grapheme_boundary};
use crate::mode::Mode;
use crate::movement::{
    Direction, Movement, first_non_whitespace_char, line_char_len, move_horizontally,
    move_next_long_word_end, move_next_long_word_start, move_next_word_end, move_next_word_start,
    move_prev_long_word_start, move_prev_word_start, move_vertically,
};
use crate::search::find_nth_char;
use crate::selection::{Range, Selection};
use crate::transaction::Transaction;
use anyhow::Result;
use ropey::RopeSlice;
use smallvec::SmallVec;
use std::path::Path;

struct HistoryEntry {
    forward: Transaction,
    inverse: Transaction,
}

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
    pub registers: Vec<String>,
    pub undo_stack: Vec<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
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
            registers: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        })
    }

    fn yank_ranges(&mut self) {
        let rope = self.document.rope().slice(..);
        self.registers = self
            .selection
            .ranges()
            .iter()
            .map(|r| r.min_width_1(rope).slice(rope).to_string())
            .collect();
    }

    pub fn yank(&mut self) {
        self.yank_ranges();
        self.enter_normal();
    }

    pub fn delete_selections(&mut self) {
        self.yank_ranges();
        let rope = self.document.rope().slice(..);
        let tx = Transaction::change_by_selection(self.document.rope(), &self.selection, |r| {
            let r = r.min_width_1(rope);
            (r.from(), r.to(), None)
        });
        self.apply_transaction(tx);
        self.enter_normal();
    }

    pub fn change_selections(&mut self) {
        self.delete_selections();
        self.mode = Mode::Insert;
    }

    pub fn paste(&mut self, after: bool) {
        if self.registers.is_empty() {
            return;
        }

        let rope = self.document.rope().slice(..);
        let vals = self.registers.clone();
        let mut carets: SmallVec<[Range; 1]> = SmallVec::new();
        let mut offset = 0usize;
        let tx = Transaction::change_by_selection(self.document.rope(), &self.selection, |r| {
            let pos = if after {
                r.min_width_1(rope).to()
            } else {
                r.from()
            };
            let value = vals[carets.len().min(vals.len() - 1)].clone();
            let len = value.chars().count();
            let anchor = pos + offset;
            carets.push(Range::new(anchor, anchor + len));
            offset += len;
            (pos, pos, Some(value))
        });

        let sel = Selection::new(carets, self.selection.primary_index());
        self.apply_transaction(tx.with_selection(sel));
        self.enter_normal();
    }

    pub fn insert_char(&mut self, c: char) {
        let rope = self.document.rope();
        let tx = Transaction::change_by_selection(rope, &self.selection, |r| {
            let pos = r.head;
            (pos, pos, Some(c.to_string()))
        });
        self.apply_transaction(tx);
    }

    pub fn insert_text(&mut self, text: &str) {
        let rope = self.document.rope();
        let tx = Transaction::change_by_selection(rope, &self.selection, |r| {
            let pos = r.head;
            (pos, pos, Some(text.to_string()))
        });
        self.apply_transaction(tx);
    }

    pub fn delete_char_backward(&mut self) {
        let rope = self.document.rope().slice(..);
        let tx = Transaction::change_by_selection(self.document.rope(), &self.selection, |r| {
            let to = r.head;
            let from = prev_grapheme_boundary(rope, to);
            (from, to, None)
        });
        self.apply_transaction(tx);
    }

    pub fn apply_transaction(&mut self, tx: Transaction) {
        if tx.changes().is_empty() {
            return;
        }

        let inverse = tx
            .invert(self.document.rope())
            .with_selection(self.selection.clone());

        let new_selection = match tx.selection() {
            Some(sel) => sel.clone(),
            None => self.selection.clone().map(tx.changes()),
        };

        self.document.apply(&tx);
        let forward = tx.with_selection(new_selection.clone());
        self.selection = new_selection;

        self.undo_stack.push(HistoryEntry { forward, inverse });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop() {
            self.apply_history(&entry.inverse);
            self.redo_stack.push(entry);
        }
    }

    pub fn redo(&mut self) {
        if let Some(entry) = self.redo_stack.pop() {
            self.apply_history(&entry.forward);
            self.undo_stack.push(entry);
        }
    }

    fn apply_history(&mut self, tx: &Transaction) {
        self.document.apply(tx);
        self.selection = match tx.selection() {
            Some(sel) => sel.clone(),
            None => self.selection.clone().map(tx.changes()),
        };
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
        self.selection = self
            .selection
            .clone()
            .transform(|r| Range::new(r.to(), r.from()));
        self.mode = Mode::Insert;
    }

    pub fn enter_insert_append(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self.selection.clone().transform(|r| {
            let pos = next_grapheme_boundary(rope, r.cursor(rope));
            r.put_cursor(rope, pos, false)
        });
        self.mode = Mode::Insert;
    }

    pub fn insert_at_line_start(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self.selection.clone().transform(|r| {
            let line = rope.char_to_line(r.cursor(rope));
            let line_start = rope.line_to_char(line);
            let pos = first_non_whitespace_char(rope.line(line))
                .map(|off| line_start + off)
                .unwrap_or(line_start);
            r.put_cursor(rope, pos, false)
        });
        self.mode = Mode::Insert;
    }

    pub fn insert_at_line_end(&mut self) {
        let rope = self.document.rope().slice(..);
        self.selection = self.selection.clone().transform(|r| {
            let line = rope.char_to_line(r.cursor(rope));
            let line_start = rope.line_to_char(line);
            let pos = line_start + line_char_len(rope, line);

            r.put_cursor(rope, pos, false)
        });
        self.mode = Mode::Insert;
    }

    pub fn open_below(&mut self) {
        self.enter_insert();
        let rope = self.document.rope().slice(..);
        let mut carets = SmallVec::new();
        let tx = Transaction::change_by_selection(self.document.rope(), &self.selection, |r| {
            let line = rope.char_to_line(r.cursor(rope));
            let pos = rope.line_to_char(line) + line_char_len(rope, line);
            carets.push(Range::point(pos + 1));
            (pos, pos, Some("\n".to_string()))
        });
        let sel = Selection::new(carets, self.selection.primary_index());
        self.apply_transaction(tx.with_selection(sel));
    }

    pub fn open_above(&mut self) {
        self.enter_insert();
        let rope = self.document.rope().slice(..);
        let mut carets = SmallVec::new();
        let tx = Transaction::change_by_selection(self.document.rope(), &self.selection, |r| {
            let line = rope.char_to_line(r.cursor(rope));
            let pos = rope.line_to_char(line);
            carets.push(Range::point(pos));
            (pos, pos, Some("\n".to_string()))
        });
        let sel = Selection::new(carets, self.selection.primary_index());
        self.apply_transaction(tx.with_selection(sel));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_editor() -> Editor {
        let path = "./src/editor.rs";
        Editor::new(Path::new(&path)).unwrap()
    }

    /// Editor over a known in-memory buffer, cursor collapsed at `pos`.
    fn editor_with(text: &str, pos: usize) -> Editor {
        let mut e = create_editor();
        e.document = crate::document::Document::from_str(text);
        e.selection = Selection::point(pos);
        e.mode = Mode::Normal;
        e
    }

    fn head(e: &Editor) -> usize {
        e.selection.primary().head
    }

    fn is_point(e: &Editor) -> bool {
        let r = e.selection.primary();
        r.anchor == r.head
    }

    #[test]
    fn apply_transaction_inserts_and_moves_cursor() {
        let mut e = create_editor(); // cursor at 0
        let tx = Transaction::change_by_selection(e.document.rope(), &e.selection, |r| {
            (r.head, r.head, Some("X".into()))
        });
        e.apply_transaction(tx);
        assert_eq!(e.document.rope().char(0), 'X');
        assert_eq!(e.selection.primary().head, 1); // cursor moved past inserted char
    }

    // ---- typing ----

    #[test]
    fn insert_char_inserts_at_cursor_and_advances() {
        let mut e = editor_with("hello", 0);
        e.insert_char('X');
        assert_eq!(e.document.rope().to_string(), "Xhello");
        assert_eq!(head(&e), 1);
        assert!(is_point(&e));
    }

    #[test]
    fn delete_char_backward_removes_grapheme_before_cursor() {
        let mut e = editor_with("hello", 3); // cursor before 'l' (index 3)
        e.delete_char_backward();
        assert_eq!(e.document.rope().to_string(), "helo"); // 'l' at index 2 removed
        assert_eq!(head(&e), 2);
    }

    #[test]
    fn delete_char_backward_at_start_is_noop() {
        let mut e = editor_with("hello", 0);
        e.delete_char_backward();
        assert_eq!(e.document.rope().to_string(), "hello");
        assert_eq!(head(&e), 0);
    }

    // ---- insert-entry: i / a ----

    #[test]
    fn enter_insert_collapses_selection_to_left_edge() {
        let mut e = editor_with("hello", 0);
        e.selection = Selection::single(1, 4); // anchor 1, head 4
        e.enter_insert();
        assert_eq!(e.mode, Mode::Insert);
        assert_eq!(head(&e), 1); // head lands on the left edge (from)
    }

    #[test]
    fn append_moves_one_past_and_stays_a_point() {
        let mut e = editor_with("hello", 2);
        e.enter_insert_append();
        assert_eq!(e.mode, Mode::Insert);
        assert_eq!(head(&e), 3); // one grapheme past the cursor
        assert!(is_point(&e)); // guards the growing-selection bug
    }

    #[test]
    fn append_then_type_does_not_grow_selection() {
        let mut e = editor_with("hello", 1); // on 'e'
        e.enter_insert_append(); // caret -> 2 (after 'e')
        e.insert_char('X');
        e.insert_char('Y');
        assert_eq!(e.document.rope().to_string(), "heXYllo");
        assert_eq!(head(&e), 4);
        assert!(is_point(&e)); // caret moves, selection never grows
    }

    // ---- insert-entry: I / A ----

    #[test]
    fn insert_at_line_start_lands_on_first_non_whitespace() {
        let mut e = editor_with("    abc\ndef", 6); // line 0, on 'b'
        e.insert_at_line_start();
        assert_eq!(e.mode, Mode::Insert);
        assert_eq!(head(&e), 4); // first non-ws of the 4-space indent
    }

    #[test]
    fn insert_at_line_start_uses_the_cursors_own_line() {
        // catches the whole-doc-vs-line + minus-vs-plus bugs on a non-first line
        let mut e = editor_with("x\n   yz", 5); // line 1 (starts at char 2), on 'y'
        e.insert_at_line_start();
        assert_eq!(head(&e), 5); // line_start(2) + first_non_ws(3 spaces) = 5
    }

    #[test]
    fn insert_at_line_start_blank_line_falls_back_to_line_start() {
        let mut e = editor_with("abc", 2);
        e.insert_at_line_start();
        assert_eq!(head(&e), 0); // no indent -> column 0
    }

    #[test]
    fn insert_at_line_end_lands_before_newline() {
        let mut e = editor_with("hello\nworld", 0);
        e.insert_at_line_end();
        assert_eq!(e.mode, Mode::Insert);
        assert_eq!(head(&e), 5); // end of "hello", before the '\n'
    }

    // ---- insert-entry: o / O ----

    #[test]
    fn open_below_inserts_newline_after_line_and_moves_down() {
        let mut e = editor_with("ab\ncd", 0); // line 0
        e.open_below();
        assert_eq!(e.mode, Mode::Insert);
        assert_eq!(e.document.rope().to_string(), "ab\n\ncd");
        assert_eq!(head(&e), 3); // start of the new empty line
        assert!(is_point(&e));
    }

    #[test]
    fn open_above_inserts_newline_before_line_and_stays() {
        let mut e = editor_with("ab\ncd", 3); // line 1, on 'c'
        e.open_above();
        assert_eq!(e.mode, Mode::Insert);
        assert_eq!(e.document.rope().to_string(), "ab\n\ncd");
        assert_eq!(head(&e), 3); // caret on the new empty line above old line 1
        assert!(is_point(&e));
    }

    // ---- yank / delete / change (item 8) ----

    #[test]
    fn yank_copies_selection_to_register_and_exits_select() {
        let mut e = editor_with("hello", 0);
        e.selection = Selection::single(0, 3); // "hel"
        e.enter_select();
        e.yank();
        assert_eq!(e.registers, vec!["hel".to_string()]);
        assert_eq!(e.mode, Mode::Normal);
        assert_eq!(e.document.rope().to_string(), "hello"); // yank does not edit
    }

    #[test]
    fn yank_resting_cursor_grabs_char_under_it() {
        // min_width_1: a point cursor yanks the grapheme it sits on
        let mut e = editor_with("hello", 1); // on 'e'
        e.yank();
        assert_eq!(e.registers, vec!["e".to_string()]);
    }

    #[test]
    fn delete_selection_removes_text_fills_register_collapses() {
        let mut e = editor_with("hello", 0);
        e.selection = Selection::single(0, 3); // "hel"
        e.delete_selections();
        assert_eq!(e.document.rope().to_string(), "lo");
        assert_eq!(e.registers, vec!["hel".to_string()]);
        assert_eq!(e.mode, Mode::Normal);
        assert_eq!(head(&e), 0); // collapsed to deletion point
    }

    #[test]
    fn delete_resting_cursor_removes_one_grapheme() {
        // min_width_1: `d` on a point deletes the char under it, not nothing
        let mut e = editor_with("hello", 1); // on 'e'
        e.delete_selections();
        assert_eq!(e.document.rope().to_string(), "hllo");
    }

    #[test]
    fn change_deletes_and_enters_insert() {
        let mut e = editor_with("hello", 0);
        e.selection = Selection::single(0, 3); // "hel"
        e.change_selections();
        assert_eq!(e.document.rope().to_string(), "lo");
        assert_eq!(e.mode, Mode::Insert);
        assert_eq!(e.registers, vec!["hel".to_string()]); // change also yanks
        assert_eq!(head(&e), 0);
    }

    // ---- paste (item 8) ----

    #[test]
    fn paste_after_inserts_register_past_cursor() {
        let mut e = editor_with("XY", 0); // on 'X'
        e.registers = vec!["ab".to_string()];
        e.paste(true); // after the grapheme under cursor (to() == 1)
        assert_eq!(e.document.rope().to_string(), "XabY");
    }

    #[test]
    fn paste_before_inserts_register_at_cursor() {
        let mut e = editor_with("XY", 1); // on 'Y'
        e.registers = vec!["ab".to_string()];
        e.paste(false); // before the cursor (from() == 1)
        assert_eq!(e.document.rope().to_string(), "XabY");
    }

    #[test]
    fn paste_empty_register_is_noop() {
        let mut e = editor_with("XY", 0);
        e.paste(true);
        assert_eq!(e.document.rope().to_string(), "XY");
    }

    #[test]
    fn yank_then_paste_duplicates_text() {
        let mut e = editor_with("hello", 0);
        e.selection = Selection::single(0, 3); // "hel"
        e.yank(); // register = ["hel"], selection kept
        e.paste(true); // insert at to() == 3
        assert_eq!(e.document.rope().to_string(), "helhello");
    }

    #[test]
    fn multi_cursor_paste_offsets_each_insertion() {
        use smallvec::smallvec;
        let mut e = editor_with("ab", 0);
        e.selection = Selection::new(smallvec![Range::point(0), Range::point(1)], 0);
        e.registers = vec!["X".to_string()]; // single value -> repeats for both cursors
        e.paste(false); // before each: from() == 0 and 1 (old coords)
        assert_eq!(e.document.rope().to_string(), "XaXb");
    }

    // ---- undo / redo (item 9) ----

    #[test]
    fn undo_restores_text_and_cursor() {
        let mut e = editor_with("hello", 0);
        e.insert_char('X'); // "Xhello", cursor at 1
        e.undo();
        assert_eq!(e.document.rope().to_string(), "hello");
        assert_eq!(head(&e), 0); // pre-edit cursor restored
    }

    #[test]
    fn redo_reapplies_edit() {
        let mut e = editor_with("hello", 0);
        e.insert_char('X');
        e.undo();
        e.redo();
        assert_eq!(e.document.rope().to_string(), "Xhello");
        assert_eq!(head(&e), 1); // post-edit cursor restored
    }

    #[test]
    fn undo_redo_walks_multiple_steps() {
        let mut e = editor_with("hello", 0);
        e.insert_char('X'); // "Xhello"
        e.insert_char('Y'); // "XYhello"
        assert_eq!(e.document.rope().to_string(), "XYhello");
        e.undo();
        assert_eq!(e.document.rope().to_string(), "Xhello");
        e.undo();
        assert_eq!(e.document.rope().to_string(), "hello");
        e.redo();
        assert_eq!(e.document.rope().to_string(), "Xhello");
        e.redo();
        assert_eq!(e.document.rope().to_string(), "XYhello");
    }

    #[test]
    fn undo_of_delete_restores_deleted_text() {
        // proves the inverse captures the deleted text (delete txns don't store it)
        let mut e = editor_with("hello", 0);
        e.selection = Selection::single(0, 3); // "hel"
        e.delete_selections(); // "lo"
        assert_eq!(e.document.rope().to_string(), "lo");
        e.undo();
        assert_eq!(e.document.rope().to_string(), "hello");
        // pre-edit selection restored
        let r = e.selection.primary();
        assert_eq!((r.anchor, r.head), (0, 3));
    }

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut e = editor_with("hello", 0);
        e.insert_char('X'); // "Xhello"
        e.undo(); // "hello", redo_stack has 1 entry
        assert_eq!(e.redo_stack.len(), 1);
        e.insert_char('Y'); // new edit -> redo branch discarded
        assert!(e.redo_stack.is_empty());
        e.redo(); // no-op
        assert_eq!(e.document.rope().to_string(), "Yhello");
    }

    #[test]
    fn undo_on_empty_stack_is_noop() {
        let mut e = editor_with("hello", 0);
        e.undo();
        e.redo();
        assert_eq!(e.document.rope().to_string(), "hello");
        assert!(e.undo_stack.is_empty());
        assert!(e.redo_stack.is_empty());
    }
}
