use ropey::{RopeSlice, str_utils::byte_to_char_idx};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

#[must_use]
pub fn nth_next_grapheme_boundary(slice: &RopeSlice, char_idx: usize, n: usize) -> usize {
    debug_assert!(char_idx <= slice.len_chars());

    let mut byte_idx = slice.char_to_byte(char_idx);

    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    for _ in 0..n {
        loop {
            match gc.next_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return slice.len_chars(),
                Ok(Some(n)) => {
                    byte_idx = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    chunk_byte_idx += chunk.len();
                    let (a, _, c, _) = slice.chunk_at_byte(chunk_byte_idx);
                    chunk = a;
                    chunk_char_idx = c;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                    gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => panic!("Unreachable boundary"),
            }
        }
    }
    let tmp = byte_to_char_idx(chunk, byte_idx - chunk_byte_idx);
    chunk_char_idx + tmp
}

#[must_use]
pub fn next_grapheme_boundary(slice: &RopeSlice, char_idx: usize) -> usize {
    nth_next_grapheme_boundary(slice, char_idx, 1)
}

#[must_use]
pub fn ensure_grapheme_boundary_next(slice: &RopeSlice, char_idx: usize) -> usize {
    if char_idx == 0 {
        char_idx
    } else {
        next_grapheme_boundary(slice, char_idx - 1)
    }
}

#[must_use]
pub fn nth_prev_grapheme_boundary(slice: &RopeSlice, char_idx: usize, n: usize) -> usize {
    debug_assert!(char_idx <= slice.len_chars());

    let mut byte_idx = slice.char_to_byte(char_idx);

    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    for _ in 0..n {
        loop {
            match gc.prev_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return 0,
                Ok(Some(n)) => {
                    byte_idx = n;
                    break;
                }
                Err(GraphemeIncomplete::PrevChunk) => {
                    let (a, b, c, _) = slice.chunk_at_byte(chunk_byte_idx - 1);
                    chunk = a;
                    chunk_byte_idx = b;
                    chunk_char_idx = c;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                    gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => panic!("Unreachable boundary"),
            }
        }
    }
    let tmp = byte_to_char_idx(chunk, byte_idx - chunk_byte_idx);
    chunk_char_idx + tmp
}

#[must_use]
pub fn prev_grapheme_boundary(slice: &RopeSlice, char_idx: usize) -> usize {
    nth_prev_grapheme_boundary(slice, char_idx, 1)
}

#[must_use]
pub fn ensure_grapheme_boundary_prev(slice: &RopeSlice, char_idx: usize) -> usize {
    if char_idx == slice.len_chars() {
        char_idx
    } else {
        prev_grapheme_boundary(slice, char_idx + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn next_skips_whole_grapheme_cluster() {
        let r = Rope::from_str("a😀b"); // 1 grapheme, 1 char here, but mutilple bytes
        let s = r.slice(..);
        assert_eq!(next_grapheme_boundary(&s, 0), 1); // Pass 'a'
        assert_eq!(next_grapheme_boundary(&s, 1), 2); // Pass grapheme
    }

    #[test]
    fn combining_char_is_one_grapheme() {
        let r = Rope::from_str("e\u{0301}x"); // e + combing acute = 1 cluster = 1 cluster (2 chars)
        let s = r.slice(..);
        assert_eq!(next_grapheme_boundary(&s, 0), 2); // skips e+accent together
    }

    #[test]
    fn prev_mirrors_next() {
        let r = Rope::from_str("a😀b");
        let s = r.slice(..);
        assert_eq!(prev_grapheme_boundary(&s, 2), 1); // back over grapheme
        assert_eq!(prev_grapheme_boundary(&s, 1), 0);
    }

    #[test]
    fn boundaries_clamp_at_ends() {
        let r = Rope::from_str("ab");
        let s = r.slice(..);
        assert_eq!(next_grapheme_boundary(&s, 2), 2); // should stay at the end
        assert_eq!(prev_grapheme_boundary(&s, 0), 0); // should stay at the start
    }
}
