use floem::prelude::*;

use crate::editor::Editor;
use crate::keymap::handle_key;
use crate::ui::render::{
    FontConfig, paint_cursor, paint_selections, paint_text, plan_screen_lines,
};

pub fn editor_view(editor: RwSignal<Editor>) -> impl View {
    let font = FontConfig::default();

    let lines = canvas(move |cx, size| {
        editor.with(|ed| {
            let screen = plan_screen_lines(ed, size, &font);
            paint_selections(cx, &screen);
            paint_cursor(cx, &screen);
            paint_text(cx, &screen);
        })
    })
    .style(|s| {
        s.keyboard_navigable()
            .background(Color::from_rgb8(30, 30, 30))
            .width_full()
            .height_full()
    })
    .on_event_stop(el::KeyDown, move |_cx, KeyboardEvent { key, .. }| {
        editor.update(|e| handle_key(e, key));
    })
    .request_focus(|| {});

    lines
}
