//! The floem view + input wiring — a thin orchestrator. Reads the reactive
//! `Editor` to paint (via the pure `render::plan_screen_lines` + `paint_*` fns) and
//! routes key events back into it through `keymap::handle_key`. Deliberately holds no
//! render logic itself; it's still a `canvas()` (paint closure), not a hand-rolled
//! `impl View` — that migration waits until struct-held state (a TextLayout cache) is
//! needed (see roadmap).

use floem::prelude::*;

use crate::editor::Editor;
use crate::keymap::handle_key;
use crate::theme::Theme;
use crate::ui::render::{
    FontConfig, paint_cursor, paint_selections, paint_text, plan_screen_lines,
};

pub fn editor_view(editor: RwSignal<Editor>) -> impl View {
    let font = FontConfig::default();
    // Focus workaround: `.request_focus(when)` runs an Effect that requests focus
    // whenever `when` re-runs. A no-dep `when` fires only once at build — before the
    // OS window is ready — so the request is lost and the editor starts unfocused.
    // Bumping this signal on every `WindowGainedFocus` gives the Effect a dep, so it
    // re-requests focus once the window is live (at launch and on every alt-tab back).
    let focus_tick = RwSignal::new(0);

    let theme = Theme::default();
    // Editor background is a floem style property (not painted in the canvas), so
    // pull it out as a peniko color before `theme` moves into the paint closure.
    let bg = theme.background.bg.map(|c| c.to_peniko());

    let lines = canvas(move |cx, size| {
        editor.with(|ed| {
            let screen = plan_screen_lines(ed, size, &font, &theme);
            // Draw order: selection bg, then caret, then text on top.
            paint_selections(cx, &screen, &theme);
            paint_cursor(cx, &screen, &theme);
            paint_text(cx, &screen);
        })
    })
    .style(move |s| {
        let s = s.keyboard_navigable().width_full().height_full();
        match bg {
            Some(c) => s.background(c),
            None => s,
        }
    })
    .on_event_stop(el::KeyDown, move |_cx, KeyboardEvent { key, .. }| {
        editor.update(|e| handle_key(e, key));
    })
    .on_event_stop(el::WindowGainedFocus, move |_cx, _| {
        focus_tick.update(|n| *n += 1);
    })
    .request_focus(move || {
        focus_tick.get(); // establish the reactive dep (see focus_tick above)
    });

    lines
}
