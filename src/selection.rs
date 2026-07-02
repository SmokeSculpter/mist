use ropey::RopeSlice;
use smallvec::{SmallVec, smallvec};

use crate::{
    grapheme::{
        ensure_grapheme_boundary_next, ensure_grapheme_boundary_prev, next_grapheme_boundary,
        prev_grapheme_boundary,
    },
    movement::Direction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    anchor: usize,
    head: usize,
    goal_col: Option<usize>,
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
        if self.anchor < self.head {
            Direction::Forward
        } else {
            Direction::Backward
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
    pub fn extends(&self, from: usize, to: usize) -> Self {
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
                head: self.head.max(from),
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

    #[must_use]
    pub fn grapheme_aligned(&self, slice: RopeSlice) -> Self {
        use std::cmp::Ordering;
        let (new_anchor, new_head) = match self.anchor.cmp(&self.head) {
            Ordering::Equal => {
                let pos = ensure_grapheme_boundary_prev(&slice, self.anchor);
                (pos, pos)
            }
            Ordering::Less => (
                ensure_grapheme_boundary_prev(&slice, self.anchor),
                ensure_grapheme_boundary_next(&slice, self.head),
            ),
            Ordering::Greater => (
                ensure_grapheme_boundary_next(&slice, self.anchor),
                ensure_grapheme_boundary_prev(&slice, self.head),
            ),
        };

        Self {
            anchor: new_anchor,
            head: new_head,
            goal_col: self.goal_col,
        }
    }

    #[inline]
    #[must_use]
    pub fn mid_width_1(&self, text: &RopeSlice) -> Self {
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

    #[inline]
    #[must_use]
    pub fn cursor(&self, text: &RopeSlice) -> usize {
        if self.head > self.anchor {
            prev_grapheme_boundary(text, self.head)
        } else {
            self.head
        }
    }

    #[inline]
    #[must_use]
    pub fn put_cursor(self, text: &RopeSlice, char_idx: usize, extend: bool) -> Range {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    ranges: SmallVec<[Range; 1]>,
    primary_index: usize,
}

impl Selection {}
