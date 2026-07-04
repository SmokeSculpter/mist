use floem::kurbo::{Point, Rect};
use floem::peniko::color::AlphaColor;
use floem::text::{Affinity, Attrs, AttrsList, FamilyOwned, FontStyle};
use floem::{prelude::*, text::TextLayout};

use crate::editor::Editor;
use crate::grapheme::next_grapheme_boundary;

pub fn editor_view(editor: RwSignal<Editor>) -> impl View {
    let lines = canvas(move |cx, size| {
        editor.with(|editor_state| {
            let rope = editor_state.document.rope().slice(..);
            let mut range_idx = 0usize;

            let font_size = 16.0;
            let line_height = 24.0;

            for line in 0..editor_state.document.line_count() {
                let line_slice = editor_state.document.line(line);
                let line_text = line_slice.to_string();
                let text = line_text.trim_end_matches("\n");
                let families = [
                    FamilyOwned::Name("JetBrains Mono".to_string()),
                    FamilyOwned::Monospace,
                ];

                let attrs = Attrs::new()
                    .font_style(FontStyle::Normal)
                    .family(&families)
                    .color(AlphaColor::from_rgb8(220, 220, 220))
                    .font_size(font_size);
                let mut attrs_list = AttrsList::new(attrs);
                let line_start = editor_state.document.line_start(line);
                let line_char_len = text.chars().count();
                let mut carets = Vec::new();

                while range_idx < editor_state.selection.len() {
                    let r = editor_state.selection.ranges()[range_idx];
                    let cur = r.cursor(&rope);

                    if editor_state.document.line_idx(cur) != line {
                        break;
                    };

                    let col = cur - line_start;
                    let byte = editor_state.document.char_to_byte_in_line(col, &line_slice);
                    let next = next_grapheme_boundary(&rope, cur);
                    let next_col = (next - line_start).min(line_char_len);
                    let next_byte = editor_state
                        .document
                        .char_to_byte_in_line(next_col, &line_slice);

                    if let Some(ch) = text[byte..].chars().next() {
                        let end = byte + ch.len_utf8();
                        attrs_list.add_span(
                            byte..end,
                            Attrs::new()
                                .family(&[FamilyOwned::Name("JetBrains Mono".to_string())])
                                .font_size(font_size)
                                .color(AlphaColor::from_rgb8(30, 30, 30)),
                        );
                    }
                    carets.push((byte, next_byte));
                    range_idx += 1;
                }

                let drawn_line = TextLayout::new_with_text(text, attrs_list, None);
                let y_offset = line as f64 * line_height;
                for (byte, next_byte) in carets {
                    let x0 = drawn_line.cursor_point(byte, Affinity::Downstream).x;
                    let x1 = drawn_line.cursor_point(next_byte, Affinity::Downstream).x;
                    let caret_w = if x1 > x0 {
                        x1 - x0
                    } else {
                        font_size as f64 * 0.6
                    };
                    cx.fill(
                        &Rect::from_origin_size((x0, y_offset), (caret_w, line_height)),
                        Color::from_rgb8(255, 255, 255),
                        0.0,
                    );
                }

                drawn_line.draw(cx, Point::new(0.0, y_offset));
            }
        })
    })
    .style(|s| {
        s.background(Color::from_rgb8(30, 30, 30))
            .width_full()
            .height_full()
    });

    lines
}
