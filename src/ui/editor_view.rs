use floem::kurbo::Point;
use floem::peniko::color::AlphaColor;
use floem::text::{Attrs, AttrsList, FontStyle};
use floem::{prelude::*, text::TextLayout};

use crate::document::Document;

pub fn editor_view(document: RwSignal<Document>) -> impl View {
    let lines = canvas(move |cx, size| {
        document.with(|doc_state| {
            for line in 0..doc_state.line_count() {
                let line_text = doc_state.line(line);
                let text = line_text.trim_end_matches("\n");

                let attrs = Attrs::new()
                    .font_style(FontStyle::Normal)
                    .color(AlphaColor::from_rgb8(220, 220, 220))
                    .font_size(16.0);
                let attrs_list = AttrsList::new(attrs);
                let drawn_line = TextLayout::new_with_text(text, attrs_list, None);

                drawn_line.draw(cx, Point::new(0.0, line as f64 * 24.0));
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
