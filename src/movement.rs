use crate::{
    grapheme::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary},
    selection::Range,
};
use ropey::RopeSlice;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Movement {
    Move,
    Extend,
}

pub fn move_horizontally(
    slice: &RopeSlice,
    range: Range,
    direction: Direction,
    count: usize,
    behavior: Movement,
) -> Range {
    let pos = range.cursor(slice);
    let new_pos = match direction {
        Direction::Forward => nth_next_grapheme_boundary(slice, pos, count),
        Direction::Backward => nth_prev_grapheme_boundary(slice, pos, count),
    };

    range.put_cursor(slice, new_pos, behavior == Movement::Extend)
}

pub fn move_vertically(
    slice: &RopeSlice,
    range: Range,
    direction: Direction,
    count: usize,
    behavior: Movement,
) -> Range {
    let pos = range.cursor(slice);
    let line_idx = slice.char_to_line(pos);
    let line_start = slice.line_to_char(line_idx);
    let col = pos - line_start;

    let goal = range.goal_col().unwrap_or(col);

    let new_line_idx = match direction {
        Direction::Forward => (line_idx + count).min(slice.len_lines() - 1),
        Direction::Backward => line_idx.saturating_sub(count),
    };

    let new_line_start = slice.line_to_char(new_line_idx);
    let new_col = goal.min(line_char_len(slice, new_line_idx));
    let new_pos = new_line_start + new_col;

    range
        .put_cursor(slice, new_pos, behavior == Movement::Extend)
        .with_goal_col(goal)
}

// Helpers
fn line_char_len(slice: &RopeSlice, line_idx: usize) -> usize {
    let line = slice.line(line_idx);
    let char_len = line.len_chars();

    if char_len > 0 && line.char(char_len - 1) == '\n' {
        char_len - 1
    } else {
        char_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::Range;
    use ropey::Rope;

    #[test]
    fn move_right_from_point() {
        let r = Rope::from_str("hello");
        let s = r.slice(..);
        assert_eq!(
            move_horizontally(&s, Range::point(0), Direction::Forward, 1, Movement::Move),
            Range::point(1)
        );
    }

    #[test]
    fn move_left_from_point() {
        let r = Rope::from_str("hello");
        let s = r.slice(..);
        assert_eq!(
            move_horizontally(&s, Range::point(3), Direction::Backward, 1, Movement::Move),
            Range::point(2)
        );
    }

    #[test]
    fn count_moves_multiple_graphemes() {
        let r = Rope::from_str("hello");
        let s = r.slice(..);
        assert_eq!(
            move_horizontally(&s, Range::point(0), Direction::Forward, 3, Movement::Move),
            Range::point(3)
        );
    }

    #[test]
    fn clamps_at_end() {
        let r = Rope::from_str("hello");
        let s = r.slice(..);
        assert_eq!(
            move_horizontally(&s, Range::point(5), Direction::Forward, 1, Movement::Move),
            Range::point(5)
        );
    }

    #[test]
    fn clamps_at_start() {
        let r = Rope::from_str("hello");
        let s = r.slice(..);
        assert_eq!(
            move_horizontally(&s, Range::point(0), Direction::Backward, 1, Movement::Move),
            Range::point(0)
        );
    }

    #[test]
    fn crosses_line_boundary() {
        let r = Rope::from_str("ab\ncd");
        let s = r.slice(..);
        assert_eq!(
            move_horizontally(&s, Range::point(1), Direction::Forward, 2, Movement::Move),
            Range::point(3)
        );
    }

    #[test]
    fn steps_over_whole_grapheme_cluster() {
        let r = Rope::from_str("e\u{0301}x");
        let s = r.slice(..);
        assert_eq!(
            move_horizontally(&s, Range::point(0), Direction::Forward, 1, Movement::Move),
            Range::point(2)
        );
    }

    #[test]
    fn move_collapses_extend_grows() {
        let r = Rope::from_str("hello world");
        let s = r.slice(..);
        let start = Range::point(2);

        let moved = move_horizontally(&s, start, Direction::Forward, 3, Movement::Move);
        assert_eq!(moved, Range::point(5));

        let extended = move_horizontally(&s, start, Direction::Forward, 3, Movement::Extend);
        assert_eq!((extended.anchor, extended.head), (2, 6));
    }

    #[test]
    fn extend_then_move_from_nonempty_range() {
        let r = Rope::from_str("hello world");
        let s = r.slice(..);
        let sel = Range::new(2, 6);
        let moved = move_horizontally(&s, sel, Direction::Forward, 1, Movement::Move);
        assert_eq!(moved, Range::point(6));
    }

    // ----- Vertical Movement -----
    #[test]
    fn down_preserves_column() {
        let r = Rope::from_str("hello\nhi\nworld\n");
        let s = r.slice(..);
        // from 'e' (col 1, line 0), j -> line 1 col 1 = 'i' (7)
        let res = move_vertically(&s, Range::point(1), Direction::Forward, 1, Movement::Move);
        assert_eq!(res.head, 7);
        assert_eq!(res.goal_col(), Some(1)); // goal carried forward
    }

    #[test]
    fn up_preserves_column() {
        let r = Rope::from_str("hello\nhi\nworld\n");
        let s = r.slice(..);
        // from 'r' (line 2, col 2 = 11), k -> line 1, col min(2,2)=2 -> pos 8 (end of "hi")
        let res = move_vertically(&s, Range::point(11), Direction::Backward, 1, Movement::Move);
        assert_eq!(res.head, 8);
    }

    #[test]
    fn sticky_column_survives_short_line() {
        let r = Rope::from_str("hello\nhi\nworld\n");
        let s = r.slice(..);
        // start at 'o' (line 0, col 4 = 4)
        let a = move_vertically(&s, Range::point(4), Direction::Forward, 1, Movement::Move);
        // line 1 "hi" only has 2 cols -> clamps to end (pos 8), but goal stays 4
        assert_eq!(a.head, 8);
        assert_eq!(a.goal_col(), Some(4));
        // j again onto "world": goal 4 restores -> col 4 = 'd' (13), NOT the clamped col 2
        let b = move_vertically(&s, a, Direction::Forward, 1, Movement::Move);
        assert_eq!(b.head, 13); // regression guard: == 11 if goal isn't carried
    }

    #[test]
    fn clamps_at_last_line() {
        let r = Rope::from_str("hello\nhi\nworld\n");
        let s = r.slice(..);
        // from line 2 col 0 (9), j x5 -> clamps to line 3 (empty), col 0 -> pos 15
        let res = move_vertically(&s, Range::point(9), Direction::Forward, 5, Movement::Move);
        assert_eq!(res.head, 15); // no panic, lands on empty last line
    }

    #[test]
    fn clamps_at_first_line() {
        let r = Rope::from_str("hello\nhi\nworld\n");
        let s = r.slice(..);
        // from line 1 col 1 (7), k x5 -> line 0, col 1 -> pos 1
        let res = move_vertically(&s, Range::point(7), Direction::Backward, 5, Movement::Move);
        assert_eq!(res.head, 1);
    }

    #[test]
    fn count_moves_multiple_lines() {
        let r = Rope::from_str("hello\nhi\nworld\n");
        let s = r.slice(..);
        // from line 0 col 0, j x2 -> line 2 col 0 = 9
        let res = move_vertically(&s, Range::point(0), Direction::Forward, 2, Movement::Move);
        assert_eq!(res.head, 9);
    }

    #[test]
    fn vertical_move_collapses_extend_grows() {
        let r = Rope::from_str("hello\nhi\nworld\n");
        let s = r.slice(..);
        let start = Range::point(0);
        // Move: collapses to a cursor on line 1 col 0 (6)
        let moved = move_vertically(&s, start, Direction::Forward, 1, Movement::Move);
        assert_eq!(moved.head, 6);
        // Extend: anchor stays 0, head covers through the target grapheme
        let ext = move_vertically(&s, start, Direction::Forward, 1, Movement::Extend);
        assert_eq!((ext.anchor, ext.head), (0, 7));
    }
}
