//! Character classification + the word-motion scanner. Word motions (w/b/e/W/B/E)
//! are defined by transitions between character *categories* — a boundary is where
//! the category changes. Ported from Helix's `chars.rs`/`movement.rs`.

use crate::movement::{WordMotionTarget, reached_target};
use ropey::iter::Chars;

use crate::line_ending::LineEnding;
use crate::selection::Range;

/// The category of a char for word-motion purposes. A "word" motion stops wherever
/// two adjacent chars fall in different categories (see `movement::is_word_boundary`).
#[derive(Eq, PartialEq, Debug)]
pub enum CharCategory {
    WhiteSpace,
    Eol,
    Word,
    Punctuation,
    Unknown,
}

/// Bucket a char into its `CharCategory`. Order matters: line endings are checked
/// before generic whitespace since a `\n` is both, but movements treat it as Eol.
#[inline]
pub fn categorize_char(ch: char) -> CharCategory {
    if char_is_line_ending(ch) {
        CharCategory::Eol
    } else if ch.is_whitespace() {
        CharCategory::WhiteSpace
    } else if char_is_word(ch) {
        CharCategory::Word
    } else if char_is_punctuation(ch) {
        CharCategory::Punctuation
    } else {
        CharCategory::Unknown
    }
}

pub fn char_is_line_ending(ch: char) -> bool {
    LineEnding::from_char(ch).is_some()
}

#[inline]
pub fn char_is_punctuation(ch: char) -> bool {
    use unicode_general_category::{GeneralCategory, get_general_category};

    matches!(
        get_general_category(ch),
        GeneralCategory::OtherPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::MathSymbol
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ModifierSymbol
    )
}

/// Word chars: alphanumeric plus `_` (so `foo_bar` is one word, like most editors).
#[inline]
pub fn char_is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Extension trait on the rope's char iterator that walks one word motion.
pub trait CharHelpers {
    fn range_to_target(&mut self, target: WordMotionTarget, origin: Range) -> Range;
}

impl CharHelpers for Chars<'_> {
    /// Scan from the iterator's current position to the next `target` boundary,
    /// returning the `Range` (anchor..head) covering the traversed span. The core
    /// of every word motion; `movement::word_move` sets up the iterator and calls
    /// this once per count. Handles both directions by reversing the iterator and
    /// flipping the index-advance step. Ported near-verbatim from Helix.
    fn range_to_target(&mut self, target: WordMotionTarget, origin: Range) -> Range {
        let is_prev = matches!(
            target,
            WordMotionTarget::PrevWordStart | WordMotionTarget::PrevLongWordStart
        );

        // Reverse the iterator if needed for the motion direction.
        if is_prev {
            self.reverse();
        }

        // Function to advance index in the appropriate motion direction.
        let advance: &dyn Fn(&mut usize) = if is_prev {
            &|idx| *idx = idx.saturating_sub(1)
        } else {
            &|idx| *idx += 1
        };

        // Initialize state variables.
        let mut anchor = origin.anchor;
        let mut head = origin.head;
        let mut prev_ch = {
            let ch = self.prev();
            if ch.is_some() {
                self.next();
            }
            ch
        };

        // Skip any initial newline characters.
        while let Some(ch) = self.next() {
            if char_is_line_ending(ch) {
                prev_ch = Some(ch);
                advance(&mut head);
            } else {
                self.prev();
                break;
            }
        }
        if prev_ch.map(char_is_line_ending).unwrap_or(false) {
            anchor = head;
        }

        // Find our target position(s).
        let head_start = head;
        #[allow(clippy::while_let_on_iterator)] // Clippy's suggestion to fix doesn't work here.
        while let Some(next_ch) = self.next() {
            if prev_ch.is_none() || reached_target(target, prev_ch.unwrap(), next_ch) {
                if head == head_start {
                    anchor = head;
                } else {
                    break;
                }
            }
            prev_ch = Some(next_ch);
            advance(&mut head);
        }

        // Un-reverse the iterator if needed.
        if is_prev {
            self.reverse();
        }

        Range::new(anchor, head)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorize() {
        assert_eq!(categorize_char('a'), CharCategory::Word);
        assert_eq!(categorize_char('_'), CharCategory::Word);
        assert_eq!(categorize_char('7'), CharCategory::Word);
        assert_eq!(categorize_char(' '), CharCategory::WhiteSpace);
        assert_eq!(categorize_char('\t'), CharCategory::WhiteSpace);
        assert_eq!(categorize_char('\n'), CharCategory::Eol);
        assert_eq!(categorize_char('.'), CharCategory::Punctuation);
        assert_eq!(categorize_char(','), CharCategory::Punctuation);
    }
    #[test]
    fn word_classification() {
        assert!(char_is_word('z'));
        assert!(char_is_word('_'));
        assert!(!char_is_word('.'));
        assert!(!char_is_word(' '));
    }
    #[test]
    fn punctuation_classification() {
        assert!(char_is_punctuation('.'));
        assert!(char_is_punctuation('+')); // MathSymbol in your impl
        assert!(!char_is_punctuation('a'));
        assert!(!char_is_punctuation(' '));
    }
}
