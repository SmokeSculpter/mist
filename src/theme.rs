//! Editor color themes. A `Theme` is a set of fully-specified UI styles; the app
//! holds one and the render pass reads it. Built-in themes are picked by name
//! (`Theme::from_name`) — the Lua config (v2 item 3) will pass that name; unset or
//! unknown falls back to `Theme::default()` (one_dark).
//!
//! `Style` keeps `Option` fg/bg because tree-sitter highlight styles (v2 item 8) are
//! *partial* and layer over the base — `None` means "leave the channel untouched".
//! The built-in UI themes below specify every channel they use.

/// An RGB color. Mist is GUI-only, so there are no ANSI/indexed/reset variants
/// (Helix's `Color` enum carries those for the TUI). Convert to floem's color at
/// the paint boundary via `to_peniko`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_peniko(self) -> floem::peniko::Color {
        floem::peniko::Color::from_rgb8(self.r, self.g, self.b)
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub struct Modifier: u8 {
        const BOLD = 1 << 0;
        const ITALIC = 1 << 1;
    }
}

/// A styled attribute. `None` fg/bg = unset (inherit the layer below); modifiers
/// are additive. Built-in UI themes fully specify the channels they use; the
/// `Option` seam is for the layered syntax-highlight styles added in v2 item 8.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub modifiers: Modifier,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            modifiers: Modifier::empty(),
        }
    }

    pub const fn bg(mut self, c: Color) -> Self {
        self.bg = Some(c);
        self
    }

    pub const fn fg(mut self, c: Color) -> Self {
        self.fg = Some(c);
        self
    }
}

/// The UI color set the render pass reads. Each field is a fully-specified
/// `Style`. `cursor` is the block caret: `bg` fills the caret, `fg` recolors the
/// glyph sitting under it (Helix's "reversed" cursor).
#[derive(Clone, Debug)]
pub struct Theme {
    pub background: Style,
    pub text: Style,
    pub cursor: Style,
    pub selection: Style,
}

impl Theme {
    /// Resolve a built-in theme by name. Unknown -> `None` (caller falls back to
    /// the default). Names match the Lua config string, e.g. `theme = "gruvbox"`.
    pub fn from_name(name: &str) -> Option<Theme> {
        match name {
            "one-dark" | "onedark" => Some(Self::one_dark()),
            "gruvbox" => Some(Self::gruvbox()),
            "catppuccin-mocha" | "catppuccin_mocha" => Some(Self::catppuccin_mocha()),
            _ => None,
        }
    }

    /// One Dark. Palette from Helix `runtime/themes/onedark.toml`.
    pub fn one_dark() -> Theme {
        Theme {
            background: Style::new().bg(Color::rgb(0x28, 0x2C, 0x34)),
            text: Style::new().fg(Color::rgb(0xAB, 0xB2, 0xBF)),
            cursor: Style::new()
                .bg(Color::rgb(0xAB, 0xB2, 0xBF))
                .fg(Color::rgb(0x28, 0x2C, 0x34)),
            selection: Style::new().bg(Color::rgb(0x5C, 0x63, 0x70)),
        }
    }

    /// Gruvbox (dark, standard contrast). Palette from Helix `runtime/themes/gruvbox.toml`.
    pub fn gruvbox() -> Theme {
        Theme {
            background: Style::new().bg(Color::rgb(0x28, 0x28, 0x28)),
            text: Style::new().fg(Color::rgb(0xEB, 0xDB, 0xB2)),
            cursor: Style::new()
                .bg(Color::rgb(0xBD, 0xAE, 0x93))
                .fg(Color::rgb(0x3C, 0x38, 0x36)),
            selection: Style::new().bg(Color::rgb(0x66, 0x5C, 0x54)),
        }
    }

    /// Catppuccin Mocha. Palette from Helix `runtime/themes/catppuccin_mocha.toml`.
    pub fn catppuccin_mocha() -> Theme {
        Theme {
            background: Style::new().bg(Color::rgb(0x1E, 0x1E, 0x2E)),
            text: Style::new().fg(Color::rgb(0xCD, 0xD6, 0xF4)),
            cursor: Style::new()
                .bg(Color::rgb(0xF5, 0xE0, 0xDC))
                .fg(Color::rgb(0x1E, 0x1E, 0x2E)),
            selection: Style::new().bg(Color::rgb(0x45, 0x47, 0x5A)),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::one_dark()
    }
}
