use ropey::{Rope, RopeSlice};

use crate::selection::{Range, Selection};

pub type Change = (usize, usize, Option<String>);
pub type Deletion = (usize, usize);

pub struct Transaction {
    changes: ChangeSet,
    selection: Option<Selection>,
}

impl Transaction {
    pub fn new(doc: &Rope) -> Self {
        Self {
            changes: ChangeSet::new(doc.slice(..)),
            selection: None,
        }
    }

    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    pub fn apply(&self, doc: &mut Rope) -> bool {
        if self.changes.is_empty() {
            return true;
        }

        self.changes.apply(doc)
    }

    pub fn invert(&self, original: &Rope) -> Self {
        let changes = self.changes.invert(original);

        Self {
            changes,
            selection: None,
        }
    }

    pub fn with_selection(mut self, selection: Selection) -> Self {
        self.selection = Some(selection);
        self
    }

    pub fn change<I>(doc: &Rope, changes: I) -> Self
    where
        I: Iterator<Item = Change>,
    {
        Self::from(ChangeSet::from_changes(doc, changes))
    }

    pub fn change_by_selection<F>(doc: &Rope, selection: &Selection, f: F) -> Self
    where
        F: FnMut(&Range) -> Change,
    {
        Self::change(doc, selection.ranges().iter().map(f))
    }
}

impl From<ChangeSet> for Transaction {
    fn from(changes: ChangeSet) -> Self {
        Self {
            changes,
            selection: None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Assoc {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Retain(usize),
    Delete(usize),
    Insert(String),
}

impl Operation {
    pub fn len_chars(&self) -> usize {
        match self {
            Self::Retain(n) | Self::Delete(n) => *n,
            Self::Insert(s) => s.chars().count(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeSet {
    pub changes: Vec<Operation>,
    len: usize,
    len_after: usize,
}

impl ChangeSet {
    pub fn map_pos(&self, pos: usize, assoc: Assoc) -> usize {
        let mut old_pos = 0;
        let mut new_pos = 0;

        for change in &self.changes {
            match change {
                Operation::Retain(n) => {
                    if old_pos + n > pos {
                        return new_pos + (pos - old_pos); // Inside retained span: shift by net offset
                    }
                    old_pos += n;
                    new_pos += n;
                }
                Operation::Delete(n) => {
                    if old_pos + n > pos {
                        return new_pos; // pos was inside deleted text -> collapse to deletion point
                    }
                    old_pos += n;
                }
                Operation::Insert(s) => {
                    let ins = s.chars().count();
                    if old_pos == pos {
                        // at the insert point: Before stays, After jumps past inserted text
                        return match assoc {
                            Assoc::Before => new_pos,
                            Assoc::After => new_pos + ins,
                        };
                    }
                    new_pos += ins;
                }
            }
        }

        new_pos
    }

    #[inline]
    pub fn new(doc: RopeSlice) -> Self {
        let len = doc.len_chars();
        Self {
            changes: Vec::new(),
            len: len,
            len_after: len,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            changes: Vec::with_capacity(capacity),
            len: 0,
            len_after: 0,
        }
    }

    pub fn delete(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        self.len += n;

        if let Some(Operation::Delete(count)) = self.changes.last_mut() {
            *count += n;
        } else {
            self.changes.push(Operation::Delete(n));
        }
    }

    pub fn insert(&mut self, fragment: String) {
        if fragment.is_empty() {
            return;
        }

        self.len_after += fragment.chars().count();

        let new_last = match self.changes.as_mut_slice() {
            [.., Operation::Insert(prev)] | [.., Operation::Insert(prev), Operation::Delete(_)] => {
                prev.push_str(&fragment);
                return;
            }
            _ => Operation::Insert(fragment),
        };

        self.changes.push(new_last);
    }

    pub fn retain(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        self.len += n;
        self.len_after += n;

        if let Some(Operation::Retain(count)) = self.changes.last_mut() {
            *count += n;
        } else {
            self.changes.push(Operation::Retain(n));
        }
    }

    pub fn from_changes<I>(doc: &Rope, changes: I) -> Self
    where
        I: Iterator<Item = Change>,
    {
        let len = doc.len_chars();

        let (lower, upper) = changes.size_hint();
        let size = upper.unwrap_or(lower);
        let mut changeset = ChangeSet::with_capacity(2 * size + 1);

        let mut last = 0;

        for (from, to, tendril) in changes {
            debug_assert!(last <= from);
            debug_assert!(from <= to, "Edit end must end before it starts");

            changeset.retain(from - last);
            let span = to - from;
            match tendril {
                Some(text) => {
                    changeset.insert(text);
                    changeset.delete(span);
                }
                None => changeset.delete(span),
            }
            last = to;
        }

        changeset.retain(len - last);
        changeset
    }

    pub fn apply(&self, text: &mut Rope) -> bool {
        if text.len_chars() != self.len {
            return false;
        };

        let mut pos = 0;

        for change in &self.changes {
            match change {
                Operation::Retain(n) => {
                    pos += n;
                }
                Operation::Delete(n) => {
                    text.remove(pos..pos + *n);
                }
                Operation::Insert(s) => {
                    text.insert(pos, s);
                    pos += s.chars().count();
                }
            }
        }
        true
    }

    pub fn invert(&self, original_doc: &Rope) -> Self {
        assert!(original_doc.len_chars() == self.len);

        let mut changes = Self::with_capacity(self.changes.len());

        let mut pos = 0;

        for change in &self.changes {
            match change {
                Operation::Retain(n) => {
                    changes.retain(*n);
                    pos += n;
                }
                Operation::Delete(n) => {
                    let text = original_doc.slice(pos..pos + n).to_string();
                    changes.insert(text);
                    pos += n;
                }
                Operation::Insert(s) => {
                    let chars = s.chars().count();
                    changes.delete(chars);
                }
            }
        }
        changes
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty() || self.changes == [Operation::Retain(self.len)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cs(orig: &str, changes: Vec<Change>) -> ChangeSet {
        let rope = Rope::from_str(orig);
        ChangeSet::from_changes(&rope, changes.into_iter())
    }

    // ---- apply ----

    #[test]
    fn apply_insert() {
        let mut doc = Rope::from_str("helo");
        let set = cs("helo", vec![(3, 3, Some("l".into()))]); // insert before last char
        assert!(set.apply(&mut doc));
        assert_eq!(doc.to_string(), "hello");
    }

    #[test]
    fn apply_delete() {
        let mut doc = Rope::from_str("hello");
        let set = cs("hello", vec![(0, 1, None)]); // drop leading 'h'
        assert!(set.apply(&mut doc));
        assert_eq!(doc.to_string(), "ello");
    }

    #[test]
    fn apply_replace() {
        let mut doc = Rope::from_str("cat");
        let set = cs("cat", vec![(0, 1, Some("b".into()))]); // c -> b
        assert!(set.apply(&mut doc));
        assert_eq!(doc.to_string(), "bat");
    }

    #[test]
    fn apply_multi_change_shifts_correctly() {
        // two edits in one set; second `from` is in OLD-doc coords
        let mut doc = Rope::from_str("a.b.c");
        let set = cs(
            "a.b.c",
            vec![(1, 2, Some("!".into())), (3, 4, Some("?".into()))],
        );
        assert!(set.apply(&mut doc));
        assert_eq!(doc.to_string(), "a!b?c");
    }

    #[test]
    fn apply_rejects_wrong_length_doc() {
        let mut doc = Rope::from_str("different length");
        let set = cs("cat", vec![(0, 1, Some("b".into()))]);
        assert!(!set.apply(&mut doc)); // len guard trips, doc untouched
        assert_eq!(doc.to_string(), "different length");
    }

    #[test]
    fn empty_changeset_is_identity() {
        let mut doc = Rope::from_str("unchanged");
        let set = cs("unchanged", vec![]); // just a retain over the whole doc
        assert!(set.apply(&mut doc));
        assert_eq!(doc.to_string(), "unchanged");
        assert!(set.is_empty());
    }

    // ---- invert roundtrip: apply(change) then apply(invert) == original ----

    fn assert_roundtrip(orig: &str, changes: Vec<Change>) {
        let original = Rope::from_str(orig);
        let set = ChangeSet::from_changes(&original, changes.into_iter());
        let inverse = set.invert(&original); // must be built from the pre-edit doc

        let mut doc = original.clone();
        assert!(set.apply(&mut doc), "forward apply failed");
        assert!(inverse.apply(&mut doc), "inverse apply failed");
        assert_eq!(doc, original, "roundtrip did not restore original");
    }

    #[test]
    fn roundtrip_insert() {
        assert_roundtrip("helo", vec![(3, 3, Some("l".into()))]);
    }

    #[test]
    fn roundtrip_delete() {
        assert_roundtrip("hello", vec![(0, 1, None)]);
    }

    #[test]
    fn roundtrip_replace() {
        assert_roundtrip("cat", vec![(0, 1, Some("bhat".into()))]); // grow: 1 -> 4 chars
    }

    #[test]
    fn roundtrip_multi_change() {
        assert_roundtrip("a.b.c", vec![(1, 2, Some("!".into())), (3, 4, None)]);
    }

    #[test]
    fn roundtrip_multibyte() {
        // char indices, not bytes: 'é' and 'ü' are one char each
        assert_roundtrip("café", vec![(3, 4, Some("ü".into()))]);
    }

    // ---- builder coalescing + bookkeeping ----

    #[test]
    fn builders_coalesce() {
        let mut set = ChangeSet::with_capacity(4);
        set.retain(2);
        set.retain(3); // merges into one Retain(5)
        set.delete(1);
        set.delete(1); // merges into one Delete(2)
        assert_eq!(
            set.changes,
            vec![Operation::Retain(5), Operation::Delete(2)]
        );
    }

    #[test]
    fn zero_length_ops_are_noops() {
        let mut set = ChangeSet::with_capacity(4);
        set.retain(0);
        set.delete(0);
        set.insert(String::new());
        assert!(set.changes.is_empty());
    }

    #[test]
    fn from_changes_retains_gaps() {
        // edit at [2,3) over "abcde" -> retain 2, delete/insert, retain to end
        let set = cs("abcde", vec![(2, 3, Some("X".into()))]);
        assert_eq!(
            set.changes,
            vec![
                Operation::Retain(2),
                Operation::Insert("X".into()),
                Operation::Delete(1),
                Operation::Retain(2),
            ]
        );
    }

    #[test]
    fn length_invariants_hold() {
        // retain + delete == old len ; retain + insert == new len
        let set = cs("abcde", vec![(1, 3, Some("XYZ".into()))]); // delete 2, insert 3
        assert_eq!(set.len, 5); // original length
        assert_eq!(set.len_after, 6); // 5 - 2 + 3
    }

    // ---- map_pos ----

    #[test]
    fn map_pos_unchanged_by_later_edit() {
        // edit sits AFTER pos -> pos is untouched
        let set = cs("abcde", vec![(3, 4, Some("X".into()))]);
        assert_eq!(set.map_pos(1, Assoc::After), 1);
    }

    #[test]
    fn map_pos_insert_shifts_later_positions() {
        // insert "XY" at 0, then 'c' (old pos 2) must move right by 2 -> catches the
        // missing `new_pos += ins` bug
        let set = cs("abc", vec![(0, 0, Some("XY".into()))]);
        assert_eq!(set.map_pos(2, Assoc::After), 4);
        assert_eq!(set.map_pos(2, Assoc::Before), 4); // assoc only matters AT the insert point
    }

    #[test]
    fn map_pos_at_insert_point_respects_assoc() {
        // insert "XY" at pos 1
        let set = cs("abc", vec![(1, 1, Some("XY".into()))]);
        assert_eq!(set.map_pos(1, Assoc::Before), 1); // stay before inserted text
        assert_eq!(set.map_pos(1, Assoc::After), 3); // jump past inserted text
    }

    #[test]
    fn map_pos_inside_deletion_collapses() {
        // delete chars [1,4) of "abcde"; positions 1,2,3 all collapse to deletion point (1)
        let set = cs("abcde", vec![(1, 4, None)]);
        assert_eq!(set.map_pos(1, Assoc::After), 1);
        assert_eq!(set.map_pos(2, Assoc::After), 1);
        assert_eq!(set.map_pos(3, Assoc::After), 1);
    }

    #[test]
    fn map_pos_after_deletion_shifts_down() {
        // delete [1,3) of "abcde"; 'd' (old pos 3) is the boundary, 'e' (old 4) -> 2
        let set = cs("abcde", vec![(1, 3, None)]);
        assert_eq!(set.map_pos(3, Assoc::After), 1); // first char past the deletion
        assert_eq!(set.map_pos(4, Assoc::After), 2);
    }

    #[test]
    fn map_pos_replace_start() {
        // replace [1,2) "b" with "XYZ" -> [Retain(1), Insert("XYZ"), Delete(1), Retain(3)]
        // pos at replace start hits the Insert arm first (canonical order matters)
        let set = cs("abcd", vec![(1, 2, Some("XYZ".into()))]);
        assert_eq!(set.map_pos(1, Assoc::Before), 1);
        assert_eq!(set.map_pos(1, Assoc::After), 4); // past inserted "XYZ"
        assert_eq!(set.map_pos(2, Assoc::After), 4); // char after replaced text
    }

    #[test]
    fn map_pos_start_and_end_of_doc() {
        let set = cs("abc", vec![(1, 1, Some("Z".into()))]);
        assert_eq!(set.map_pos(0, Assoc::After), 0); // before any edit
        assert_eq!(set.map_pos(3, Assoc::After), 4); // doc end, pushed by the insert
    }

    #[test]
    fn map_pos_multibyte() {
        // "café" -> insert "ü" before 'é' (old pos 3); 'é' shifts to 4
        let set = cs("café", vec![(3, 3, Some("ü".into()))]);
        assert_eq!(set.map_pos(3, Assoc::After), 4);
        assert_eq!(set.map_pos(3, Assoc::Before), 3);
    }

    // ---- Transaction ----

    #[test]
    fn transaction_from_changeset_has_no_selection() {
        let set = cs("abc", vec![(0, 0, Some("X".into()))]);
        let tx = Transaction::from(set);
        assert!(tx.selection().is_none());
    }

    #[test]
    fn transaction_apply_empty_is_noop() {
        // an all-retain (empty) transaction leaves the doc untouched and reports success
        let mut doc = Rope::from_str("abc");
        let tx = Transaction::new(&doc); // no changes recorded
        assert!(tx.apply(&mut doc));
        assert_eq!(doc.to_string(), "abc");
    }

    #[test]
    fn transaction_apply_change() {
        let mut doc = Rope::from_str("abc");
        let tx = Transaction::change(&doc, vec![(1, 1, Some("Z".into()))].into_iter());
        assert!(tx.apply(&mut doc));
        assert_eq!(doc.to_string(), "aZbc");
    }

    #[test]
    fn transaction_invert_roundtrip() {
        let original = Rope::from_str("hello world");
        let tx = Transaction::change(&original, vec![(0, 5, Some("bye".into()))].into_iter());
        let inverse = tx.invert(&original);

        let mut doc = original.clone();
        assert!(tx.apply(&mut doc));
        assert_eq!(doc.to_string(), "bye world");
        assert!(inverse.apply(&mut doc));
        assert_eq!(doc, original);
    }

    #[test]
    fn with_selection_stores_it() {
        let doc = Rope::from_str("abc");
        let sel = Selection::point(2);
        let tx = Transaction::new(&doc).with_selection(sel.clone());
        assert_eq!(tx.selection(), Some(&sel));
    }

    #[test]
    fn change_by_selection_single_cursor_insert() {
        // one cursor at pos 1; insert "X" there
        let doc = Rope::from_str("abc");
        let sel = Selection::point(1);
        let tx = Transaction::change_by_selection(&doc, &sel, |r| (r.head, r.head, Some("X".into())));

        let mut d = doc.clone();
        assert!(tx.apply(&mut d));
        assert_eq!(d.to_string(), "aXbc");
    }

    #[test]
    fn change_by_selection_multi_cursor_insert() {
        use smallvec::smallvec;
        // two cursors at 0 and 3 in "abcdef"; insert "X" at each.
        // changes are emitted in OLD-doc coords, sorted -> from_changes stays valid
        let doc = Rope::from_str("abcdef");
        let sel = Selection::new(smallvec![Range::point(0), Range::point(3)], 0);
        let tx = Transaction::change_by_selection(&doc, &sel, |r| (r.head, r.head, Some("X".into())));

        let mut d = doc.clone();
        assert!(tx.apply(&mut d));
        assert_eq!(d.to_string(), "XabcXdef");
    }

    #[test]
    fn change_by_selection_delete_before_cursor() {
        // backspace-style: delete the char before each cursor
        let doc = Rope::from_str("abc");
        let sel = Selection::point(2); // between 'b' and 'c'
        let tx = Transaction::change_by_selection(&doc, &sel, |r| (r.head - 1, r.head, None));

        let mut d = doc.clone();
        assert!(tx.apply(&mut d));
        assert_eq!(d.to_string(), "ac"); // 'b' removed
    }
}
