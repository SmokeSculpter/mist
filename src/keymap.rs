//! Pure key dispatch: `handle_key` maps a keypress to an `Editor` mutation, gated by
//! the current mode. Kept free of floem wiring (the `on_event_stop`/focus plumbing
//! lives in `editor_view`) so it can be unit-tested headlessly. Count prefixes and
//! pending-input motions (f/t, g-prefix) will grow this into a small input-state
//! machine; see the roadmap.

use crate::{
    command::{Context, STATIC_COMMAND_MAP},
    editor::{Editor, PendingFind},
    mode::Mode,
    movement::{Direction, Movement},
};
use floem::prelude::{Key, NamedKey};

/// Interpret one keypress in the current mode and apply it to `editor`. Normal =
/// motions collapse (`Movement::Move`); Select = the same motions extend
/// (`Movement::Extend` / the `extend_*` word variants); Insert = Esc back to Normal.
pub fn handle_key_temp(ctx: &mut Context, key: &Key) {
    if let Some(callback) = ctx.on_next_key.take() {
        callback(ctx, key);
    }

    if ctx.editor.mode != Mode::Insert {
        if let Key::Character(ch) = key {
            if let Ok(n) = ch.parse::<usize>() {
                if ctx.count.is_some() {
                    ctx.append_count_digit(n);
                    return;
                }
            }
        }
    }

    let run_command = |str: &str, ctx: &mut Context| {
        if let Some(cmd) = STATIC_COMMAND_MAP.get(str) {
            (cmd.fun)(ctx);
        }
    };

    match ctx.editor.mode {
        Mode::Normal => match key {
            Key::Character(ch) if ch == "h" => run_command("move_cursor_left", ctx),
            Key::Character(ch) if ch == "l" => run_command("move_cursor_right", ctx),
            Key::Character(ch) if ch == "j" => run_command("move_cursor_up", ctx),
            Key::Character(ch) if ch == "k" => run_command("move_cursor_down", ctx),
            Key::Character(ch) if ch == "i" => run_command("enter_insert", ctx),
            _ => {}
        },
        Mode::Select => {}
        Mode::Insert => {}
    }
}

pub fn handle_key(editor: &mut Editor, key: &Key) {
    // Awaiting f/t/F/T target -- early return gate
    if let Some(find) = editor.pending_find.take() {
        if let Key::Character(ch) = key {
            if let Some(c) = ch.chars().next() {
                editor.find_char(&find, c);
            }
        }
        // Return if key is any non char key such as ESC
        return;
    }
    // Pending go to gate -- toggled below
    if editor.pending_goto {
        editor.pending_goto = false;
        let extend = editor.mode == Mode::Select;
        if let Key::Character(ch) = key {
            match ch.as_str() {
                "g" => editor.goto_file_start(extend),
                "e" => editor.goto_file_end(extend),
                "h" => editor.goto_line_start(extend),
                "l" => editor.goto_line_end(extend),
                _ => {}
            }
        }
        // Consume the key whether or not it was a valid goto target (like pending_find)
        return;
    }
    // Count guard
    if editor.mode != Mode::Insert {
        if let Key::Character(ch) = key {
            if let Ok(n) = ch.parse::<usize>() {
                if editor.count.is_some() || n > 0 {
                    editor.push_count_digit(n);
                    return;
                }
            }
        }
    }

    let n = editor.take_count();

    match editor.mode {
        Mode::Normal => match key {
            Key::Character(ch) if ch == "h" => {
                editor.move_h(Direction::Backward, n, Movement::Move)
            }
            Key::Character(ch) if ch == "l" => editor.move_h(Direction::Forward, n, Movement::Move),
            Key::Character(ch) if ch == "j" => editor.move_v(Direction::Forward, n, Movement::Move),
            Key::Character(ch) if ch == "k" => {
                editor.move_v(Direction::Backward, n, Movement::Move)
            }
            Key::Character(ch) if ch == "i" => editor.enter_insert(),
            Key::Character(ch) if ch == "a" => editor.enter_insert_append(),
            Key::Character(ch) if ch == "I" => editor.insert_at_line_start(),
            Key::Character(ch) if ch == "A" => editor.insert_at_line_end(),
            Key::Character(ch) if ch == "o" => editor.open_below(),
            Key::Character(ch) if ch == "O" => editor.open_above(),
            Key::Character(ch) if ch == "v" => editor.enter_select(),
            Key::Character(ch) if ch == "w" => editor.move_next_word_start(n),
            Key::Character(ch) if ch == "W" => editor.move_next_long_word_start(n),
            Key::Character(ch) if ch == "e" => editor.move_next_word_end(n),
            Key::Character(ch) if ch == "E" => editor.move_next_long_word_end(n),
            Key::Character(ch) if ch == "b" => editor.move_prev_word_start(n),
            Key::Character(ch) if ch == "B" => editor.move_prev_long_word_start(n),
            Key::Character(ch) if ch == "d" => editor.delete_selections(),
            Key::Character(ch) if ch == "c" => editor.change_selections(),
            Key::Character(ch) if ch == "y" => editor.yank(),
            Key::Character(ch) if ch == "p" => editor.paste(true),
            Key::Character(ch) if ch == "P" => editor.paste(false),
            Key::Character(ch) if ch == "u" => editor.undo(),
            Key::Character(ch) if ch == "U" => editor.redo(),
            Key::Character(ch) if ch == "f" => {
                editor.pending_find = Some(PendingFind {
                    count: n,
                    dir: Direction::Forward,
                    inclusive: true,
                    extend: false,
                })
            }
            Key::Character(ch) if ch == "t" => {
                editor.pending_find = Some(PendingFind {
                    count: n,
                    dir: Direction::Forward,
                    inclusive: false,
                    extend: false,
                })
            }
            Key::Character(ch) if ch == "F" => {
                editor.pending_find = Some(PendingFind {
                    count: n,
                    dir: Direction::Backward,
                    inclusive: true,
                    extend: false,
                })
            }
            Key::Character(ch) if ch == "T" => {
                editor.pending_find = Some(PendingFind {
                    count: n,
                    dir: Direction::Backward,
                    inclusive: false,
                    extend: false,
                })
            }
            Key::Character(ch) if ch == "g" => editor.pending_goto = true,
            Key::Named(NamedKey::Home) => editor.goto_line_start(false),
            Key::Named(NamedKey::End) => editor.goto_line_end(false),
            _ => {}
        },
        Mode::Insert => match key {
            Key::Named(NamedKey::Escape) => editor.enter_normal(),
            Key::Named(NamedKey::Home) => editor.goto_line_start(false),
            Key::Named(NamedKey::End) => editor.goto_line_end(false),
            Key::Named(NamedKey::Backspace) => editor.delete_char_backward(),
            Key::Named(NamedKey::Enter) => editor.insert_text("\n"),
            Key::Named(NamedKey::Tab) => editor.insert_text("\t"),
            Key::Character(ch) => {
                if let Some(c) = ch.chars().next() {
                    editor.insert_char(c);
                }
            }
            _ => {}
        },
        Mode::Select => match key {
            Key::Named(NamedKey::Escape) => editor.enter_normal(),
            Key::Character(ch) if ch == "v" => editor.enter_normal(),
            Key::Character(ch) if ch == "i" => editor.enter_insert(),
            Key::Character(ch) if ch == "a" => editor.enter_insert_append(),
            Key::Character(ch) if ch == "I" => editor.insert_at_line_start(),
            Key::Character(ch) if ch == "A" => editor.insert_at_line_end(),
            Key::Character(ch) if ch == "o" => editor.open_below(),
            Key::Character(ch) if ch == "O" => editor.open_above(),
            Key::Character(ch) if ch == "d" => editor.delete_selections(),
            Key::Character(ch) if ch == "c" => editor.change_selections(),
            Key::Character(ch) if ch == "y" => editor.yank(),
            Key::Character(ch) if ch == "p" => editor.paste(true),
            Key::Character(ch) if ch == "P" => editor.paste(false),
            Key::Character(ch) if ch == "h" => {
                editor.move_h(Direction::Backward, n, Movement::Extend)
            }
            Key::Character(ch) if ch == "l" => {
                editor.move_h(Direction::Forward, n, Movement::Extend)
            }
            Key::Character(ch) if ch == "j" => {
                editor.move_v(Direction::Forward, n, Movement::Extend)
            }
            Key::Character(ch) if ch == "k" => {
                editor.move_v(Direction::Backward, n, Movement::Extend)
            }
            Key::Character(ch) if ch == "w" => editor.extend_next_word_start(n),
            Key::Character(ch) if ch == "W" => editor.extend_next_long_word_start(n),
            Key::Character(ch) if ch == "e" => editor.extend_next_word_end(n),
            Key::Character(ch) if ch == "E" => editor.extend_next_long_word_end(n),
            Key::Character(ch) if ch == "b" => editor.extend_prev_word_start(n),
            Key::Character(ch) if ch == "B" => editor.extend_prev_long_word_start(n),
            Key::Character(ch) if ch == "f" => {
                editor.pending_find = Some(PendingFind {
                    count: n,
                    dir: Direction::Forward,
                    inclusive: true,
                    extend: true,
                })
            }
            Key::Character(ch) if ch == "t" => {
                editor.pending_find = Some(PendingFind {
                    count: n,
                    dir: Direction::Forward,
                    inclusive: false,
                    extend: true,
                })
            }
            Key::Character(ch) if ch == "F" => {
                editor.pending_find = Some(PendingFind {
                    count: n,
                    dir: Direction::Backward,
                    inclusive: true,
                    extend: true,
                })
            }
            Key::Character(ch) if ch == "T" => {
                editor.pending_find = Some(PendingFind {
                    count: n,
                    dir: Direction::Backward,
                    inclusive: false,
                    extend: true,
                })
            }
            Key::Character(ch) if ch == "g" => editor.pending_goto = true,
            Key::Named(NamedKey::Home) => editor.goto_line_start(true),
            Key::Named(NamedKey::End) => editor.goto_line_end(true),
            _ => {}
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{editor::Editor, keymap::handle_key, mode::Mode};
    use floem::prelude::{Key, NamedKey};
    use std::path::Path;

    fn create_editor() -> Editor {
        let path = "./src/document.rs";
        Editor::new(Path::new(&path)).unwrap()
    }

    /// Editor over a known in-memory buffer, cursor at char 0. Deterministic — use
    /// this instead of `create_editor` when a test asserts exact positions.
    fn editor_with(text: &str) -> Editor {
        let mut e = create_editor();
        e.document = crate::document::Document::from_str(text);
        e.selection = crate::selection::Selection::point(0);
        e.mode = Mode::Normal;
        e
    }

    #[test]
    fn f_finds_char_forward() {
        let mut e = create_editor(); // buffer starts at char 0
        let start = e.selection.primary().head;
        handle_key(&mut e, &Key::Character("f".into()));
        handle_key(&mut e, &Key::Character("e".into())); // jump to first 'e'
        assert!(e.selection.primary().cursor(e.document.rope().slice(..)) > start);
        assert!(e.pending_find.is_none()); // consumed
    }

    #[test]
    fn count_then_f_finds_nth() {
        // "beebee": 'e' at char indices 1, 2, 4, 5. `fe` lands on the 1st; `2fe` on the 2nd.
        let mut one = editor_with("beebee");
        handle_key(&mut one, &Key::Character("f".into()));
        handle_key(&mut one, &Key::Character("e".into()));
        let p1 = one
            .selection
            .primary()
            .cursor(one.document.rope().slice(..));

        let mut two = editor_with("beebee");
        handle_key(&mut two, &Key::Character("2".into()));
        handle_key(&mut two, &Key::Character("f".into()));
        handle_key(&mut two, &Key::Character("e".into()));
        let p2 = two
            .selection
            .primary()
            .cursor(two.document.rope().slice(..));

        assert_eq!(p1, 1); // first 'e'
        assert_eq!(p2, 2); // second 'e' — the count took effect
        assert!(p2 > p1);
        assert!(two.pending_find.is_none()); // find target consumed
        assert_eq!(two.count, None); // count consumed
    }

    #[test]
    fn w_extends_in_select_mode() {
        let mut e = create_editor();
        e.enter_select();
        let anchor = e.selection.primary().anchor;
        handle_key(&mut e, &Key::Character("w".to_string()));
        let r = e.selection.primary();
        assert_eq!(r.anchor, anchor); // anchor stays put
        assert!(r.head > anchor); // head extends over the word
    }

    #[test]
    fn l_extends_selection_in_select_mode() {
        let mut e = create_editor();
        e.enter_select();
        let anchor = e.selection.primary().anchor;
        handle_key(&mut e, &Key::Character("l".to_string()));
        let r = e.selection.primary();
        assert_eq!(r.anchor, anchor);
        assert!(r.head > anchor);
    }

    #[test]
    fn w_moves_selection_forward() {
        let mut e = create_editor();
        let before = e.selection.primary().head;
        handle_key(&mut e, &Key::Character("w".to_string()));
        assert!(e.selection.primary().head > before);
    }
    #[test]
    fn e_moves_selection_forward() {
        let mut e = create_editor();
        let before = e.selection.primary().head;
        handle_key(&mut e, &Key::Character("e".to_string()));
        assert!(e.selection.primary().head > before);
    }
    #[test]
    fn b_moves_selection_backward() {
        let mut e = create_editor();
        handle_key(&mut e, &Key::Character("w".to_string()));
        handle_key(&mut e, &Key::Character("w".to_string()));
        let mid = e.selection.primary().head;
        handle_key(&mut e, &Key::Character("b".to_string()));
        assert!(e.selection.primary().head < mid);
    }

    #[test]
    fn i_enters_insert_mode_in_normal_mode() {
        let mut editor = create_editor();
        handle_key(&mut editor, &Key::Character("i".to_string()));
        assert_eq!(editor.mode, Mode::Insert);
    }

    #[test]
    fn i_enters_insert_mode_in_select_mode() {
        let mut editor = create_editor();
        editor.enter_select();
        handle_key(&mut editor, &Key::Character("i".to_string()));
        assert_eq!(editor.mode, Mode::Insert);
    }

    #[test]
    fn esc_enters_normal_mode_in_insert_mode() {
        let mut editor = create_editor();
        editor.enter_insert();
        handle_key(&mut editor, &Key::Named(NamedKey::Escape));
        assert_eq!(editor.mode, Mode::Normal);
    }

    #[test]
    fn v_enters_select_mode_in_normal_mode() {
        let mut editor = create_editor();
        handle_key(&mut editor, &Key::Character("v".to_string()));
        assert_eq!(editor.mode, Mode::Select);
    }

    #[test]
    fn v_enters_normal_mode_in_select_mode() {
        let mut editor = create_editor();
        editor.enter_select();
        handle_key(&mut editor, &Key::Character("v".to_string()));
        assert_eq!(editor.mode, Mode::Normal);
    }

    #[test]
    fn digits_accumulate_into_count() {
        let mut e = create_editor();
        handle_key(&mut e, &Key::Character("1".into()));
        handle_key(&mut e, &Key::Character("2".into()));
        assert_eq!(e.count, Some(12));
    }

    #[test]
    fn count_then_motion_moves_n_lines_and_clears() {
        let mut e = create_editor();
        let line0 = e.document.line_idx(e.selection.primary().head);
        handle_key(&mut e, &Key::Character("3".into()));
        handle_key(&mut e, &Key::Character("j".into()));
        let line_after = e.document.line_idx(e.selection.primary().head);
        assert_eq!(line_after - line0, 3);
        assert_eq!(e.count, None); // consumed
    }

    #[test]
    fn leading_zero_is_not_a_count() {
        let mut e = create_editor();
        handle_key(&mut e, &Key::Character("0".into()));
        assert_eq!(e.count, None); // fell through to command dispatch (0 unbound -> no-op)
    }

    #[test]
    fn gg_goes_to_file_start() {
        let mut e = create_editor();
        handle_key(&mut e, &Key::Character("j".into())); // move off 0
        handle_key(&mut e, &Key::Character("g".into()));
        handle_key(&mut e, &Key::Character("g".into()));
        assert_eq!(e.selection.primary().cursor(e.document.rope().slice(..)), 0);
        assert!(!e.pending_goto);
    }
}
