//! The positional state of the editor: `Selection` (a set of `Range`s with one
//! primary) built on the block-cursor `Range`. Ported from Helix `selection.rs`.
//! This is the foundation everything depends on — movement transforms ranges,
//! render draws them, and every command reads/writes the selection.

use ropey::RopeSlice;
use smallvec::{SmallVec, smallvec};

use crate::{
    grapheme::{
        ensure_grapheme_boundary_next, ensure_grapheme_boundary_prev, next_grapheme_boundary,
        prev_grapheme_boundary,
    },
    movement::Direction,
};

/// A single selection range, half-open `[from, to)` in char indices.
///
/// `anchor` is the fixed end, `head` the moving end — so `anchor > head` means the
/// range points backward, and a bare cursor is `anchor == head`. In the block-cursor
/// model the "cursor" is the grapheme just before `head` (see [`Range::cursor`]).
/// `goal_col` is the sticky target column for vertical motion: it survives passing
/// through short lines so `j`/`k` return to the original column (Helix behavior).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub anchor: usize,
    pub head: usize,
    pub goal_col: Option<usize>,
}

impl Range {
    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            goal_col: None,
        }
    }

    pub fn point(head: usize) -> Self {
        Self::new(head, head)
    }

    pub fn goal_col(&self) -> Option<usize> {
        self.goal_col
    }

    #[must_use]
    pub fn with_goal_col(mut self, goal_col: usize) -> Self {
        self.goal_col = Some(goal_col);
        self
    }

    /// The lower bound of the range, regardless of direction. Pair with [`Range::to`]
    /// to get sorted `[from, to)` bounds without caring which end is the head.
    #[inline]
    #[must_use]
    pub fn from(&self) -> usize {
        self.anchor.min(self.head)
    }

    #[inline]
    #[must_use]
    pub fn to(&self) -> usize {
        self.anchor.max(self.head)
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.to() - self.from()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    #[inline]
    #[must_use]
    pub fn direction(&self) -> Direction {
        if self.head < self.anchor {
            Direction::Backward
        } else {
            Direction::Forward
        }
    }

    #[inline]
    #[must_use]
    pub fn flip(&self) -> Self {
        Self {
            anchor: self.head,
            head: self.anchor,
            goal_col: self.goal_col,
        }
    }

    #[inline]
    #[must_use]
    pub fn with_direction(self, direction: Direction) -> Self {
        if self.direction() == direction {
            self
        } else {
            self.flip()
        }
    }

    /// Do these two ranges touch? Shared left edge counts (so adjacent zero-width
    /// points at the same spot merge), but merely-adjacent non-empty ranges do not.
    /// Drives the merge step in [`Selection::normalize`].
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.from() == other.from() || (self.to() > other.from() && other.to() > self.from())
    }

    #[must_use]
    pub fn contains_range(&self, other: Self) -> bool {
        self.from() <= other.from() && self.to() >= other.to()
    }

    #[must_use]
    pub fn contains(&self, pos: usize) -> bool {
        self.from() <= pos && pos < self.to()
    }

    #[must_use]
    pub fn extend(&self, from: usize, to: usize) -> Self {
        debug_assert!(from <= to);
        if self.anchor <= self.head {
            Self {
                anchor: self.anchor.min(from),
                head: self.head.max(to),
                goal_col: None,
            }
        } else {
            Self {
                anchor: self.anchor.max(to),
                head: self.head.min(from),
                goal_col: None,
            }
        }
    }

    #[must_use]
    pub fn merge(&self, other: Self) -> Self {
        if self.anchor > self.head && other.anchor > other.head {
            Self {
                anchor: self.anchor.max(other.anchor),
                head: self.head.min(other.head),
                goal_col: None,
            }
        } else {
            Self {
                anchor: self.from().min(other.from()),
                head: self.to().max(other.to()),
                goal_col: None,
            }
        }
    }

    #[inline]
    pub fn slice<'a>(&self, text: RopeSlice<'a>) -> RopeSlice<'a> {
        text.slice(self.from()..self.to())
    }

    /// Snap both ends to grapheme boundaries so a range can never split a cluster
    /// (e.g. an `e` + combining accent). Direction-aware: the outer edges expand and
    /// the inner edges contract as appropriate. Drops `goal_col` if the anchor moved.
    #[must_use]
    pub fn grapheme_aligned(&self, slice: RopeSlice) -> Self {
        use std::cmp::Ordering;
        let (new_anchor, new_head) = match self.anchor.cmp(&self.head) {
            Ordering::Equal => {
                let pos = ensure_grapheme_boundary_prev(slice, self.anchor);
                (pos, pos)
            }
            Ordering::Less => (
                ensure_grapheme_boundary_prev(slice, self.anchor),
                ensure_grapheme_boundary_next(slice, self.head),
            ),
            Ordering::Greater => (
                ensure_grapheme_boundary_next(slice, self.anchor),
                ensure_grapheme_boundary_prev(slice, self.head),
            ),
        };

        Self {
            anchor: new_anchor,
            head: new_head,
            goal_col: if new_anchor == self.anchor {
                self.goal_col
            } else {
                None
            },
        }
    }

    /// Widen a zero-width (point) range to cover one grapheme, so the block cursor
    /// always has a cell to sit on. Non-empty ranges are returned unchanged.
    #[inline]
    #[must_use]
    pub fn min_width_1(&self, text: RopeSlice) -> Self {
        if self.head == self.anchor {
            Self {
                anchor: self.anchor,
                head: next_grapheme_boundary(text, self.head),
                goal_col: self.goal_col,
            }
        } else {
            *self
        }
    }

    /// The block-cursor position: the char index of the grapheme the cursor sits on.
    /// For a forward range that's one grapheme back from `head` (since `head` is the
    /// exclusive end); for a backward range or a point it's `head` itself. This is
    /// what render draws the block over.
    #[inline]
    #[must_use]
    pub fn cursor(&self, text: RopeSlice) -> usize {
        if self.head > self.anchor {
            prev_grapheme_boundary(text, self.head)
        } else {
            self.head
        }
    }

    /// Move the cursor to `char_idx`. With `extend == false` the range collapses to a
    /// point there (Normal-mode motion); with `extend == true` the anchor is kept and
    /// the head moves, growing the selection (Select-mode / word-extend). The
    /// grapheme-boundary juggling handles crossing the anchor: when the head flips
    /// past the anchor, the anchor shifts by one grapheme so the covered cell stays
    /// consistent. Mirrors Helix `put_cursor` exactly.
    #[inline]
    #[must_use]
    pub fn put_cursor(self, text: RopeSlice, char_idx: usize, extend: bool) -> Range {
        if extend {
            let anchor = if self.head >= self.anchor && char_idx < self.anchor {
                next_grapheme_boundary(text, self.anchor)
            } else if self.head < self.anchor && char_idx >= self.anchor {
                prev_grapheme_boundary(text, self.anchor)
            } else {
                self.anchor
            };
            if anchor <= char_idx {
                Range::new(anchor, next_grapheme_boundary(text, char_idx))
            } else {
                Range::new(anchor, char_idx)
            }
        } else {
            Range::point(char_idx)
        }
    }
}

/// One or more `Range`s with a designated primary (the one commands act on / the
/// caret you drive). Invariant, maintained by [`Selection::normalize`]: ranges are
/// sorted by `from` and non-overlapping (overlaps are merged). The render relies on
/// the sorted order for its lockstep caret walk. `SmallVec<[Range; 1]>` keeps the
/// common single-cursor case allocation-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    ranges: SmallVec<[Range; 1]>,
    primary_index: usize,
}

impl Selection {
    #[inline]
    #[must_use]
    pub fn primary(&self) -> Range {
        self.ranges[self.primary_index]
    }

    #[inline]
    #[must_use]
    pub fn primary_mut(&mut self) -> &mut Range {
        &mut self.ranges[self.primary_index]
    }

    pub fn ranges(&self) -> &[Range] {
        &self.ranges
    }

    pub fn primary_index(&self) -> usize {
        self.primary_index
    }

    pub fn set_primary_index(&mut self, idx: usize) {
        assert!(idx < self.ranges.len());
        self.primary_index = idx;
    }

    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    pub fn single(anchor: usize, head: usize) -> Self {
        Self {
            ranges: smallvec![Range::new(anchor, head)],
            primary_index: 0,
        }
    }

    pub fn point(pos: usize) -> Self {
        Self::single(pos, pos)
    }

    #[must_use]
    pub fn new(ranges: SmallVec<[Range; 1]>, primary_idx: usize) -> Self {
        assert!(!ranges.is_empty());
        debug_assert!(primary_idx < ranges.len());
        Self {
            ranges,
            primary_index: primary_idx,
        }
        .normalize()
    }

    pub fn into_single(self) -> Self {
        if self.ranges.len() == 1 {
            self
        } else {
            Self {
                ranges: smallvec![self.ranges[self.primary_index]],
                primary_index: 0,
            }
        }
    }

    pub fn push(mut self, range: Range) -> Self {
        self.ranges.push(range);
        self.primary_index = self.ranges.len() - 1;
        self.normalize()
    }

    pub fn remove(mut self, index: usize) -> Self {
        assert!(self.ranges.len() > 1, "Can't remove last Range");
        self.ranges.remove(index);
        if index < self.primary_index || self.primary_index == self.ranges.len() {
            self.primary_index -= 1;
        };

        self
    }

    pub fn replace(mut self, index: usize, range: Range) -> Self {
        self.ranges[index] = range;
        self.normalize()
    }

    /// Map every range through `f` and re-normalize. This is how motions apply — a
    /// movement is a `Range -> Range` function run over each cursor, and normalize
    /// then merges any that collided. The `Copy` bound on `Direction`/`Movement`
    /// exists so the same closure can run across all ranges here.
    pub fn transform<F: FnMut(Range) -> Range>(mut self, mut f: F) -> Self {
        for range in self.ranges.iter_mut() {
            *range = f(*range);
        }
        self.normalize()
    }

    pub fn transform_iter<F, I>(mut self, f: F) -> Self
    where
        F: FnMut(Range) -> I,
        I: Iterator<Item = Range>,
    {
        self.ranges = self.ranges.into_iter().flat_map(f).collect();
        self.normalize()
    }

    pub fn ensure_invariants(self, text: RopeSlice) -> Self {
        self.transform(|r| r.min_width_1(text).grapheme_aligned(text))
    }

    pub fn cursors(self, text: RopeSlice) -> Self {
        self.transform(|r| Range::point(r.cursor(text)))
    }

    /// Restore the invariant: sort ranges by `from`, merge any that overlap, and
    /// track which merged range the primary ended up in (so `primary_index` still
    /// points at it afterward). No-op for a single range.
    fn normalize(mut self) -> Self {
        if self.ranges.len() < 2 {
            return self;
        }
        let mut primary = self.ranges[self.primary_index];
        self.ranges.sort_unstable_by_key(Range::from);
        self.ranges.dedup_by(|curr, prev| {
            if prev.overlaps(curr) {
                let merged = prev.merge(*curr);
                if prev == &primary || curr == &primary {
                    primary = merged;
                }
                *prev = merged;
                true
            } else {
                false
            }
        });
        self.primary_index = self.ranges.iter().position(|&r| r == primary).unwrap();
        self
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::Direction;
    use ropey::Rope;
    use smallvec::smallvec;

    fn fmt(sel: &Selection) -> String {
        sel.ranges()
            .iter()
            .map(|r| format!("{}/{}", r.anchor, r.head))
            .collect::<Vec<_>>()
            .join(",")
    }

    // ----- Range -----

    #[test]
    fn from_to_len_empty() {
        let r = Range::new(6, 3);
        assert_eq!(r.from(), 3);
        assert_eq!(r.to(), 6);
        assert_eq!(r.len(), 3);
        assert!(!r.is_empty());
        assert!(Range::point(4).is_empty());
    }

    #[test]
    fn direction_and_empty_is_forward() {
        assert_eq!(Range::new(2, 6).direction(), Direction::Forward);
        assert_eq!(Range::new(6, 2).direction(), Direction::Backward);
        // Empty range should be 1 selction forard
        assert_eq!(Range::point(3).direction(), Direction::Forward);
    }

    #[test]
    fn overlap_returns_correctly() {
        let ov = |a: (usize, usize), b: (usize, usize)| {
            Range::new(a.0, a.1).overlaps(&Range::new(b.0, b.1))
        };

        // Adjacement non-zero width ranges do not overlap
        assert!(!ov((0, 3), (3, 6)));
        assert!(!ov((3, 0), (6, 3)));
        // Overlapping in the middle
        assert!(ov((0, 4), (3, 6)));
        assert!(ov((6, 3), (4, 0)));
        // Zero width does not overlap adjacement edge, but shares the left edge of another
        assert!(!ov((0, 3), (3, 3)));
        assert!(ov((1, 4), (1, 1)));
        assert!(ov((3, 3), (1, 4)));
        // Different points never overlap but the same ones do
        assert!(!ov((1, 1), (2, 2)));
        assert!(ov((1, 1), (1, 1)));
    }

    #[test]
    fn merge_forward_and_backward() {
        assert_eq!(Range::new(0, 3).merge(Range::new(2, 6)), Range::new(0, 6));
        let m = Range::new(6, 2).merge(Range::new(4, 1));

        assert_eq!((m.anchor, m.head), (6, 1));
    }

    #[test]
    fn cursor_is_block_left_edge() {
        let rope = Rope::from_str("hello world");
        let s = rope.slice(..);
        assert_eq!(Range::point(3).cursor(s), 3); // point: head itself
        assert_eq!(Range::new(2, 6).cursor(s), 5); // forward: one grapheme back from head
        assert_eq!(Range::new(6, 2).cursor(s), 2); // backward: head itself
    }

    #[test]
    fn put_cursor_move_collapses() {
        let rope = Rope::from_str("hello world");
        let s = rope.slice(..);
        assert_eq!(Range::new(0, 5).put_cursor(s, 8, false), Range::point(8));
    }

    #[test]
    fn put_cursor_extend_1_width() {
        let rope = Rope::from_str("hello world");
        let s = rope.slice(..);
        // extend a cursor at 2 forward to 5 -> covers char 5 (head = next boundary)
        let f = Range::point(2).put_cursor(s, 5, true);
        assert_eq!((f.anchor, f.head), (2, 6));
        // extend a cursor at 5 backward to 2 -> anchor jumps forward one, head lands on 2
        let b = Range::point(5).put_cursor(s, 2, true);
        assert_eq!((b.anchor, b.head), (6, 2));
    }

    #[test]
    fn min_width_1() {
        let rope = Rope::from_str("hello");
        let s = rope.slice(..);
        let w = Range::point(3).min_width_1(s);
        assert_eq!((w.anchor, w.head), (3, 4)); // point widened
        let nz = Range::new(1, 3);
        assert_eq!(nz.min_width_1(s), nz); // non-empty untouched
    }

    #[test]
    fn grapheme_aligned_snaps_off_combining() {
        // "e" + combining acute = one grapheme (2 chars); index 1 is mid-cluster
        let rope = Rope::from_str("e\u{0301}x");
        let s = rope.slice(..);
        let a = Range::point(1).grapheme_aligned(s);
        assert_eq!((a.anchor, a.head), (0, 0));
    }

    // ---- Selection ----

    #[test]
    #[should_panic]
    fn new_empty_panics() {
        let _ = Selection::new(smallvec![], 0);
    }

    #[test]
    fn point_and_single() {
        assert_eq!(fmt(&Selection::point(4)), "4/4");
        assert_eq!(fmt(&Selection::single(2, 5)), "2/5");
        assert_eq!(Selection::point(4).len(), 1);
    }

    #[test]
    fn normalize_sorts_and_merges() {
        let sel = Selection::new(
            smallvec![
                Range::new(10, 12),
                Range::new(6, 7),
                Range::new(4, 5),
                Range::new(3, 4),
                Range::new(0, 6),
                Range::new(7, 8),
                Range::new(9, 13),
                Range::new(13, 14),
            ],
            0,
        );
        assert_eq!(fmt(&sel), "0/6,6/7,7/8,9/13,13/14");
    }

    #[test]
    fn normalize_recomputes_primary() {
        // three ranges collapse into one; primary must survive the merge
        let sel = Selection::new(
            smallvec![Range::new(0, 2), Range::new(1, 5), Range::new(4, 7)],
            2,
        );
        assert_eq!(fmt(&sel), "0/7");
        assert_eq!(sel.primary_index(), 0);
    }

    #[test]
    fn merges_adjacent_points() {
        let sel = Selection::new(
            smallvec![
                Range::new(10, 12),
                Range::new(12, 12),
                Range::new(12, 12),
                Range::new(10, 10),
                Range::new(8, 10),
            ],
            0,
        );
        assert_eq!(fmt(&sel), "8/10,10/12,12/12");
    }

    #[test]
    fn push_sets_primary_and_normalizes() {
        let sel = Selection::point(0).push(Range::new(5, 8));
        assert_eq!(fmt(&sel), "0/0,5/8");
        assert_eq!(sel.primary(), Range::new(5, 8)); // pushed range is primary
    }

    #[test]
    fn transform_maps_every_range() {
        let sel = Selection::single(0, 0).push(Range::new(4, 4));
        let shifted = sel.transform(|r| Range::point(r.head + 1));
        assert_eq!(fmt(&shifted), "1/1,5/5");
    }

    #[test]
    fn into_single_keeps_primary() {
        let sel = Selection::point(0).push(Range::new(5, 8)); // primary = 5/8
        assert_eq!(fmt(&sel.into_single()), "5/8");
    }
}
