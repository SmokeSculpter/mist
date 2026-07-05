//! Character search over the rope. Currently just `find_nth_char`, the primitive
//! behind the `f/t/F/T` find-char motions (see `Editor::find_char`). Full `/` `?`
//! `n` `N` text search is a later addition. Ported from Helix `search.rs`.

use crate::movement::Direction;
use ropey::RopeSlice;

/// Char index of the `n`th occurrence of `needle` starting at `pos`, scanning
/// `direction`. `None` if there aren't `n` of them (or `n == 0`). Ported from
/// Helix `search::find_nth_char`.
pub fn find_nth_char(
    n: usize,
    text: RopeSlice,
    needle: char,
    mut pos: usize,
    direction: Direction,
) -> Option<usize> {
    if n == 0 {
        return None;
    }

    let mut n = n;
    let mut chars = text.get_chars_at(pos)?;
    match direction {
        Direction::Forward => loop {
            let c = chars.next()?;
            if c == needle {
                n -= 1;
                if n == 0 {
                    return Some(pos);
                }
            }
            pos += 1;
        },
        Direction::Backward => loop {
            let c = chars.prev()?;
            pos -= 1;
            if c == needle {
                n -= 1;
                if n == 0 {
                    return Some(pos);
                }
            }
        },
    }
}
