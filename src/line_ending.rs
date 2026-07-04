#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum LineEnding {
    Crlf, // Cariage return followed by line feed

    LF, // Line feed

    VT, // Vertical tab

    FF, // Form feed

    CR, // Carriage return

    Nel, // Next line

    LS, // Line seperator

    PS, // Paragraph seperator
}

impl LineEnding {
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
