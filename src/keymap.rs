//! Pure key dispatch: `handle_key` maps a keypress to an `Editor` mutation, gated by
//! the current mode. Kept free of floem wiring (the `on_event_stop`/focus plumbing
//! lives in `editor_view`) so it can be unit-tested headlessly. Count prefixes and
//! pending-input motions (f/t, g-prefix) will grow this into a small input-state
//! machine; see the roadmap.

use crate::{
    editor::Editor,
    mode::Mode,
    movement::{Direction, Movement},
};
use floem::prelude::{Key, NamedKey};

/// Interpret one keypress in the current mode and apply it to `editor`. Normal =
/// motions collapse (`Movement::Move`); Select = the same motions extend
/// (`Movement::Extend` / the `extend_*` word variants); Insert = Esc back to Normal.
pub fn handle_key(editor: &mut Editor, key: &Key) {
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
            Key::Character(ch) if ch == "v" => editor.enter_select(),
            Key::Character(ch) if ch == "w" => editor.move_next_word_start(n),
            Key::Character(ch) if ch == "W" => editor.move_next_long_word_start(n),
            Key::Character(ch) if ch == "e" => editor.move_next_word_end(n),
            Key::Character(ch) if ch == "E" => editor.move_next_long_word_end(n),
            Key::Character(ch) if ch == "b" => editor.move_prev_word_start(n),
            Key::Character(ch) if ch == "B" => editor.move_prev_long_word_start(n),
            _ => {}
        },
        Mode::Insert => match key {
            Key::Named(NamedKey::Escape) => editor.enter_normal(),
            _ => {}
        },
        Mode::Select => match key {
            Key::Named(NamedKey::Escape) => editor.enter_normal(),
            Key::Character(ch) if ch == "v" => editor.enter_normal(),
            Key::Character(ch) if ch == "i" => editor.enter_insert(),
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
}
