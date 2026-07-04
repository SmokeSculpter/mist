#[derive(Eq, PartialEq, Debug)]
pub enum CharCategory {
    WhiteSpace,
    Eol,
    Word,
    Punctuation,
    Unknown,
}

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
    true
}

pub fn char_is_word(ch: char) -> bool {
    true
}

pub fn char_is_punctuation(ch: char) -> bool {
    true
}
