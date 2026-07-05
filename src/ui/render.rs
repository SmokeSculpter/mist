//! Render geometry + painting for the editor surface, split into a pure planning
//! pass and dumb consumers. `plan_screen_lines(&Editor, Size, &FontConfig)` computes
//! a `ScreenLines` (visible-line-range math + per-line `TextLayout` + caret/selection
//! rects) with no side effects — so it's headless-testable — and the `paint_*` fns
//! only fill/draw from it. All positions are char indices into the rope; floem's
//! layout APIs index by *byte* within a line, hence the `char_to_byte_in_line` hops.
//! Colors are still inline here (theme-struct extraction is v2 item 1).

use floem::Renderer;
use floem::context::PaintCx;
use floem::kurbo::{Point, Rect, Size};
use floem::peniko::Color;
use floem::peniko::color::AlphaColor;
use floem::text::{Affinity, Attrs, AttrsList, FamilyOwned, FontStyle, TextLayout};

use crate::editor::Editor;
use crate::grapheme::next_grapheme_boundary;
use crate::mode::Mode;

/// Font/metrics config for the text surface. Previously these values were
/// hardcoded inline in `editor_view.rs` (font_size 16, line_height 24,
/// JetBrains Mono / Monospace); centralized here so the measurement pass and
/// the paint pass read one source. (Colors are still inline below — see the
/// roadmap "centralize into a theme/config struct" todo.)
pub struct FontConfig {
    pub font_size: f32,
    pub line_height: f64,
    pub families: Vec<FamilyOwned>,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_height: 24.0,
            families: vec![
                FamilyOwned::Name("JetBrains Mono".to_string()),
                FamilyOwned::Monospace,
            ],
        }
    }
}

/// Everything the paint pass needs, computed once per frame. The pure
/// `plan_screen_lines` produces this; the `paint_*` functions only consume it.
/// Modeled on floem's `ScreenLines`/`LineInfo` (src/views/editor/view.rs).
pub struct ScreenLines {
    pub lines: Vec<LineInfo>,
}

/// One visible rope line, laid out with its paint geometry resolved.
pub struct LineInfo {
    pub line_idx: usize,
    /// Top y of this line in the canvas.
    pub y: f64,
    /// Shaped line text (block-cursor inverted spans already applied).
    pub layout: TextLayout,
    /// Block/bar caret rects on this line (usually 0 or 1; more with multi-cursor).
    pub caret_rects: Vec<Rect>,
    /// Highlights for current selections on this line
    pub selection_rects: Vec<Rect>,
    /// Char index of the line's first char (for selection-rect math, item 5).
    pub line_start: usize,
    /// Char count of the line excluding the trailing newline (for selection clamp).
    pub char_len: usize,
}

/// Decide what/where to draw for the visible viewport. Pure over `&Editor` +
/// metrics — returns data, draws nothing. This is the "make render testable"
/// extraction: visible-line-range math + per-line layout + caret geometry live
/// here, not in the paint closure.
pub fn plan_screen_lines(editor: &Editor, size: Size, font: &FontConfig) -> ScreenLines {
    let rope = editor.document.rope().slice(..);
    let mode = editor.mode;

    let line_count = editor.document.line_count();
    let visible = (size.height / font.line_height).ceil() as usize + 1;
    let last = visible.min(line_count);

    let mut lines = Vec::with_capacity(last);
    // Ranges are sorted by `from`, each range's cursor sits on exactly one line,
    // so a single index walks them in lockstep with the line loop.
    let mut range_idx = 0usize;

    for line in 0..last {
        let line_slice = editor.document.line(line);
        let line_text = line_slice.to_string();
        let text = line_text.trim_end_matches('\n');
        let line_start = editor.document.line_start(line);
        let char_len = text.chars().count();

        let families = font.families.as_slice();
        let attrs = Attrs::new()
            .font_style(FontStyle::Normal)
            .family(families)
            .color(AlphaColor::from_rgb8(220, 220, 220))
            .font_size(font.font_size);
        let mut attrs_list = AttrsList::new(attrs);

        let mut caret_bytes: Vec<(usize, usize)> = Vec::new();
        while range_idx < editor.selection.len() {
            let r = editor.selection.ranges()[range_idx];
            let cur = r.cursor(rope);

            if editor.document.line_idx(cur) != line {
                break;
            }

            let col = cur - line_start;
            let byte = editor.document.char_to_byte_in_line(col, line_slice);
            let next = next_grapheme_boundary(rope, cur);
            let next_col = (next - line_start).min(char_len);
            let next_byte = editor.document.char_to_byte_in_line(next_col, line_slice);

            // Invert the glyph under a block cursor so it reads against the caret.
            if mode == Mode::Normal {
                if let Some(ch) = text[byte..].chars().next() {
                    let end = byte + ch.len_utf8();
                    attrs_list.add_span(
                        byte..end,
                        Attrs::new()
                            .family(families)
                            .font_size(font.font_size)
                            .color(AlphaColor::from_rgb8(30, 30, 30)),
                    );
                }
            }
            caret_bytes.push((byte, next_byte));
            range_idx += 1;
        }

        let layout = TextLayout::new_with_text(text, attrs_list, None);
        let y = line as f64 * font.line_height;
        let true_line_height = layout.size().height;

        let line_end = line_start + char_len;
        let mut selection_rects = Vec::new();
        for r in editor.selection.ranges() {
            let seg_from = r.from().max(line_start);
            let seg_to = r.to().min(line_end);

            if seg_from < seg_to {
                let b0 = editor
                    .document
                    .char_to_byte_in_line(seg_from - line_start, line_slice);
                let b1 = editor
                    .document
                    .char_to_byte_in_line(seg_to - line_start, line_slice);
                let x0 = layout.cursor_point(b0, Affinity::Downstream).x;
                let x1 = layout.cursor_point(b1, Affinity::Downstream).x;
                // `- true_line_height * 0.25`: nudge the rect up to vertically center
                // the fill inside the `line_height` band (the shaped glyph box is
                // shorter than line_height). Same fudge is applied to caret rects.
                selection_rects.push(Rect::from_origin_size(
                    (x0, y - true_line_height * 0.25),
                    (x1 - x0, font.line_height),
                ));
            }
        }

        let mut caret_rects = Vec::with_capacity(caret_bytes.len());
        for (byte, next_byte) in caret_bytes {
            let x0 = layout.cursor_point(byte, Affinity::Downstream).x;
            let x1 = layout.cursor_point(next_byte, Affinity::Downstream).x;
            // Insert mode: a 2px bar. Normal/Select: a block the width of the grapheme
            // under it (x1 - x0). Fallback (x1 == x0, e.g. caret past line end / empty
            // line where there's no glyph to measure): a nominal ~0.6em block.
            let caret_w = if mode == Mode::Insert {
                2.0
            } else if x1 > x0 {
                x1 - x0
            } else {
                font.font_size as f64 * 0.6
            };
            caret_rects.push(Rect::from_origin_size(
                (x0, y - true_line_height * 0.25),
                (caret_w, font.line_height),
            ));
        }

        lines.push(LineInfo {
            line_idx: line,
            y,
            layout,
            caret_rects,
            selection_rects,
            line_start,
            char_len,
        });
    }

    ScreenLines { lines }
}

/// Fill block/bar caret rects. Drawn BEFORE text so glyphs sit on top.
pub fn paint_cursor(cx: &mut PaintCx, screen: &ScreenLines) {
    for line in &screen.lines {
        for rect in &line.caret_rects {
            cx.fill(rect, Color::from_rgb8(255, 255, 255), 0.0);
        }
    }
}

/// Draw each visible line's shaped text at its planned origin.
pub fn paint_text(cx: &mut PaintCx, screen: &ScreenLines) {
    for line in &screen.lines {
        line.layout.draw(cx, Point::new(0.0, line.y));
    }
}

pub fn paint_selections(cx: &mut PaintCx, screen: &ScreenLines) {
    for line in &screen.lines {
        for rect in &line.selection_rects {
            cx.fill(rect, Color::from_rgb8(38, 79, 120), 0.0);
        }
    }
}
