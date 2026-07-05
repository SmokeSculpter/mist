//! Unicode line-ending classification. Movement and word motions treat any of
//! these code points as a line boundary (via `chars::char_is_line_ending`), not
//! just `\n`. Ported from Helix so the editor handles files with non-LF endings
//! correctly. (Editing/normalization of endings is a later concern; this is the
//! read-side "is this char a line break?" table.)

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum LineEnding {
    Crlf, // Carriage return followed by line feed

    LF, // Line feed

    VT, // Vertical tab

    FF, // Form feed

    CR, // Carriage return

    Nel, // Next line

    LS, // Line separator

    PS, // Paragraph separator
}

impl LineEnding {
    /// Classify a single char as a line ending, or `None` if it isn't one.
    /// `const` so callers can use it in const contexts; the code points are the
    /// Unicode line-break set (LF, VT, FF, CR, NEL, LS, PS). Note CRLF can't be
    /// detected from one char — that's a two-char sequence handled elsewhere.
    pub const fn from_char(ch: char) -> Option<LineEnding> {
        match ch {
            '\u{000A}' => Some(LineEnding::LF),
            '\u{000B}' => Some(LineEnding::VT),
            '\u{000C}' => Some(LineEnding::FF),
            '\u{000D}' => Some(LineEnding::CR),
            '\u{0085}' => Some(LineEnding::Nel),
            '\u{2028}' => Some(LineEnding::LS),
            '\u{2029}' => Some(LineEnding::PS),
            // Not a line ending
            _ => None,
        }
    }
}
