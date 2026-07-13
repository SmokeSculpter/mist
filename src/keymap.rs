//! Pure key dispatch: `handle_key` maps a keypress to an `Editor` mutation, gated by
//! the current mode. Kept free of floem wiring (the `on_event_stop`/focus plumbing
//! lives in `editor_view`) so it can be unit-tested headlessly. Count prefixes and
//! pending-input motions (f/t, g-prefix) will grow this into a small input-state
//! machine; see the roadmap.

use crate::{
    command::{Context, STATIC_COMMAND_MAP},
    config::KeyAction,
    mode::Mode,
};
use floem::{
    imbl::HashMap,
    prelude::{Key, NamedKey},
};

pub type KeyMap = HashMap<String, fn(&mut Context)>;

struct KeyTree {
    normal_map: KeyMap,
    insert_map: KeyMap,
    select_map: KeyMap,
    pending_map: Option<KeyMap>,
}

impl KeyTree {
    pub fn default(config_keys: Vec<KeyAction>) -> Self {
        let mut normal_map: KeyMap = HashMap::new();
        let mut insert_map: KeyMap = HashMap::new();
        let mut select_map: KeyMap = HashMap::new();

        for (mode, key_set, command) in DEFAULT_KEYS.iter() {
            if let Some(cmd) = STATIC_COMMAND_MAP.get(command) {
                match *mode {
                    "insert" => {
                        insert_map.insert(key_set.to_string(), cmd.fun);
                    }
                    "normal" => {
                        normal_map.insert(key_set.to_string(), cmd.fun);
                    }
                    "select" => {
                        select_map.insert(key_set.to_string(), cmd.fun);
                    }
                    _ => panic!("Default mode typo"),
                }
            }
        }

        for key_action in config_keys.iter() {
            match key_action {
                KeyAction::Add((mode, key_set, command)) => {
                    if let Some(cmd) = STATIC_COMMAND_MAP.get(command.as_str()) {
                        match mode.as_str() {
                            "insert" => {
                                insert_map.insert(key_set.clone(), cmd.fun);
                            }
                            "normal" => {
                                normal_map.insert(key_set.clone(), cmd.fun);
                            }
                            "select" => {
                                select_map.insert(key_set.clone(), cmd.fun);
                            }
                            _ => panic!("That is not a mode"),
                        }
                    }
                }
                KeyAction::Remove((mode, key_set)) => match mode.as_str() {
                    "insert" => {
                        insert_map.remove(key_set);
                    }
                    "normal" => {
                        normal_map.remove(key_set);
                    }
                    "select" => {
                        select_map.remove(key_set);
                    }
                    _ => panic!("That is not a mode"),
                },
            }
        }

        Self {
            normal_map,
            insert_map,
            select_map,
            pending_map: None,
        }
    }

    pub fn handle_key(&self, ctx: &mut Context, key: &Key) {
        if let Some(callback) = ctx.on_next_key.take() {
            callback(ctx, key);
            return;
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
    }
}

pub fn convert_key(key: &Key) -> Option<String> {
    match key {
        Key::Character(ch) => Some(ch.to_string()),
        Key::Named(nk) => match nk {
            NamedKey::Escape => Some(String::from("<ESC>")),
            NamedKey::Home => Some(String::from("<HOME>")),
            NamedKey::Enter => Some(String::from("<ENTER>")),
            NamedKey::Backspace => Some(String::from("<BACKSPACE>")),
            NamedKey::Tab => Some(String::from("<TAB>")),
            _ => None,
        },
    }
}

/// Interpret one keypress in the current mode and apply it to `editor`. Normal =
/// motions collapse (`Movement::Move`); Select = the same motions extend
/// (`Movement::Extend` / the `extend_*` word variants); Insert = Esc back to Normal.
pub fn handle_key(ctx: &mut Context, key: &Key) {
    if let Some(callback) = ctx.on_next_key.take() {
        callback(ctx, key);
        return;
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
            Key::Character(ch) if ch == "j" => run_command("move_cursor_down", ctx),
            Key::Character(ch) if ch == "k" => run_command("move_cursor_up", ctx),
            Key::Character(ch) if ch == "i" => run_command("enter_insert", ctx),
            Key::Character(ch) if ch == "a" => run_command("enter_insert_append", ctx),
            Key::Character(ch) if ch == "I" => run_command("insert_at_line_start", ctx),
            Key::Character(ch) if ch == "A" => run_command("insert_at_line_end", ctx),
            Key::Character(ch) if ch == "o" => run_command("open_below", ctx),
            Key::Character(ch) if ch == "O" => run_command("open_above", ctx),
            Key::Character(ch) if ch == "v" => run_command("enter_select", ctx),
            Key::Character(ch) if ch == "w" => run_command("move_next_word_start", ctx),
            Key::Character(ch) if ch == "W" => run_command("move_next_long_word_start", ctx),
            Key::Character(ch) if ch == "e" => run_command("move_next_word_end", ctx),
            Key::Character(ch) if ch == "E" => run_command("move_next_long_word_end", ctx),
            Key::Character(ch) if ch == "b" => run_command("move_prev_word_start", ctx),
            Key::Character(ch) if ch == "B" => run_command("move_prev_long_word_start", ctx),
            Key::Character(ch) if ch == "d" => run_command("delete_selections", ctx),
            Key::Character(ch) if ch == "c" => run_command("change_selections", ctx),
            Key::Character(ch) if ch == "y" => run_command("yank", ctx),
            Key::Character(ch) if ch == "p" => run_command("paste_after_cursor", ctx),
            Key::Character(ch) if ch == "P" => run_command("paste_before_cursor", ctx),
            Key::Character(ch) if ch == "u" => run_command("undo", ctx),
            Key::Character(ch) if ch == "U" => run_command("redo", ctx),
            Key::Character(ch) if ch == "f" => run_command("move_on_next_char_forward", ctx),
            Key::Character(ch) if ch == "F" => run_command("move_on_next_char_backward", ctx),
            Key::Character(ch) if ch == "t" => run_command("move_before_char_forward", ctx),
            Key::Character(ch) if ch == "T" => run_command("move_before_char_backward", ctx),
            Key::Character(ch) if ch == "g" => run_command("goto_pending", ctx),
            Key::Named(NamedKey::Home) => run_command("goto_line_start", ctx),
            Key::Named(NamedKey::End) => run_command("goto_line_end", ctx),
            _ => {}
        },
        Mode::Select => match key {
            Key::Character(ch) if ch == "h" => run_command("extend_cursor_left", ctx),
            Key::Character(ch) if ch == "l" => run_command("extend_cursor_right", ctx),
            Key::Character(ch) if ch == "j" => run_command("extend_cursor_down", ctx),
            Key::Character(ch) if ch == "k" => run_command("extend_cursor_up", ctx),
            Key::Character(ch) if ch == "i" => run_command("enter_insert", ctx),
            Key::Character(ch) if ch == "a" => run_command("enter_insert_append", ctx),
            Key::Character(ch) if ch == "I" => run_command("insert_at_line_start", ctx),
            Key::Character(ch) if ch == "A" => run_command("insert_at_line_end", ctx),
            Key::Character(ch) if ch == "o" => run_command("open_below", ctx),
            Key::Character(ch) if ch == "O" => run_command("open_above", ctx),
            Key::Character(ch) if ch == "v" => run_command("enter_normal", ctx),
            Key::Character(ch) if ch == "w" => run_command("extend_next_word_start", ctx),
            Key::Character(ch) if ch == "W" => run_command("extend_next_long_word_start", ctx),
            Key::Character(ch) if ch == "e" => run_command("extend_next_word_end", ctx),
            Key::Character(ch) if ch == "E" => run_command("extend_next_long_word_end", ctx),
            Key::Character(ch) if ch == "b" => run_command("extend_prev_word_start", ctx),
            Key::Character(ch) if ch == "B" => run_command("extend_prev_long_word_start", ctx),
            Key::Character(ch) if ch == "d" => run_command("delete_selections", ctx),
            Key::Character(ch) if ch == "c" => run_command("change_selections", ctx),
            Key::Character(ch) if ch == "y" => run_command("yank", ctx),
            Key::Character(ch) if ch == "p" => run_command("paste_after_cursor", ctx),
            Key::Character(ch) if ch == "P" => run_command("paste_before_cursor", ctx),
            Key::Character(ch) if ch == "u" => run_command("undo", ctx),
            Key::Character(ch) if ch == "U" => run_command("redo", ctx),
            Key::Character(ch) if ch == "f" => run_command("extend_on_next_char_forward", ctx),
            Key::Character(ch) if ch == "F" => run_command("extend_on_next_char_backward", ctx),
            Key::Character(ch) if ch == "t" => run_command("extend_before_char_forward", ctx),
            Key::Character(ch) if ch == "T" => run_command("extend_before_char_backward", ctx),
            Key::Character(ch) if ch == "g" => run_command("goto_pending", ctx),
            Key::Named(NamedKey::Escape) => run_command("enter_normal", ctx),
            Key::Named(NamedKey::Home) => run_command("goto_line_start_extend", ctx),
            Key::Named(NamedKey::End) => run_command("goto_line_end_extend", ctx),
            _ => {}
        },
        Mode::Insert => match key {
            Key::Named(NamedKey::Escape) => run_command("enter_normal", ctx),
            Key::Named(NamedKey::Home) => run_command("goto_line_start", ctx),
            Key::Named(NamedKey::End) => run_command("goto_line_end", ctx),
            Key::Named(NamedKey::Backspace) => ctx.editor.delete_char_backward(),
            Key::Named(NamedKey::Enter) => ctx.editor.insert_text("\n"),
            Key::Named(NamedKey::Tab) => ctx.editor.insert_text("\t"),
            Key::Character(ch) => {
                if let Some(c) = ch.chars().next() {
                    ctx.editor.insert_char(c);
                }
            }
            _ => {}
        },
    }
}

static DEFAULT_KEYS: &[(&'static str, &'static str, &'static str)] = &[
    ("normal", "h", "move_cursor_left"),
    ("normal", "l", "move_cursor_right"),
    ("normal", "j", "move_cursor_down"),
    ("normal", "k", "move_cursor_up"),
    ("normal", "i", "enter_insert"),
    ("normal", "a", "enter_insert_append"),
    ("normal", "I", "insert_at_line_start"),
    ("normal", "A", "insert_at_line_end"),
    ("normal", "o", "open_below"),
    ("normal", "O", "open_above"),
    ("normal", "v", "enter_select"),
    ("normal", "w", "move_next_word_start"),
    ("normal", "W", "move_next_long_word_start"),
    ("normal", "e", "move_next_word_end"),
    ("normal", "E", "move_next_load_word_end"),
    ("normal", "b", "move_prev_word_start"),
    ("normal", "B", "move_prev_long_word_start"),
    ("normal", "d", "delete_selections"),
    ("normal", "c", "change_selections"),
    ("normal", "y", "yank"),
    ("normal", "p", "paste_after_cursor"),
    ("normal", "P", "paste_before_cursor"),
    ("normal", "u", "undo"),
    ("normal", "U", "redo"),
    ("normal", "f", "move_on_next_char_forward"),
    ("normal", "F", "move_on_next_char_backward"),
    ("normal", "t", "move_before_char_forward"),
    ("normal", "T", "move_before_char_backward"),
    ("normal", "g", "goto_pending"),
    ("normal", "<HOME>", "goto_line_start"),
    ("normal", "<END>", "goto_line_end"),
    ("select", "h", "extend_cursor_left"),
    ("select", "l", "extend_cursor_right"),
    ("select", "j", "extend_cursor_down"),
    ("select", "k", "extend_cursor_up"),
    ("select", "i", "enter_insert"),
    ("select", "a", "enter_insert_append"),
    ("select", "I", "insert_at_line_start"),
    ("select", "A", "insert_at_line_end"),
    ("select", "o", "open_below"),
    ("select", "O", "open_above"),
    ("select", "v", "enter_normal"),
    ("select", "w", "extend_next_word_start"),
    ("select", "W", "extend_next_long_word_start"),
    ("select", "e", "extend_next_word_end"),
    ("select", "E", "extend_next_long_word_end"),
    ("select", "b", "extend_prev_word_start"),
    ("select", "B", "extend_prev_long_word_start"),
    ("select", "d", "delete_selections"),
    ("select", "c", "change_selections"),
    ("select", "y", "yank"),
    ("select", "p", "paste_after_cursor"),
    ("select", "P", "paste_before_cursor"),
    ("select", "u", "undo"),
    ("select", "U", "redo"),
    ("select", "f", "extend_on_next_char_forward"),
    ("select", "F", "extend_on_next_char_backward"),
    ("select", "t", "extend_before_char_forward"),
    ("select", "T", "extend_before_char_backward"),
    ("select", "g", "goto_pending"),
    ("select", "<ESC>", "enter_normal"),
    ("select", "<HOME>", "goto_line_start"),
    ("select", "<END>", "goto_line_end"),
    ("insert", "<ESC>", "enter_normal"),
    ("insert", "<HOME>", "goto_line_start"),
    ("insert", "<END>", "goto_line_end"),
    ("insert", "<BACKSPACE>", "delete_char_backward"),
    ("insert", "<ENTER>", "insert_new_line"),
    ("insert", "<TAB>", "insert_tab"),
];

// #[cfg(test)]
// mod tests {
//     use crate::{editor::Editor, keymap::handle_key, mode::Mode};
//     use floem::prelude::{Key, NamedKey};
//     use std::path::Path;

//     fn create_editor() -> Editor {
//         let path = "./src/document.rs";
//         Editor::new(Path::new(&path)).unwrap()
//     }

//     /// Editor over a known in-memory buffer, cursor at char 0. Deterministic — use
//     /// this instead of `create_editor` when a test asserts exact positions.
//     fn editor_with(text: &str) -> Editor {
//         let mut e = create_editor();
//         e.document = crate::document::Document::from_str(text);
//         e.selection = crate::selection::Selection::point(0);
//         e.mode = Mode::Normal;
//         e
//     }

//     #[test]
//     fn f_finds_char_forward() {
//         let mut e = create_editor(); // buffer starts at char 0
//         let start = e.selection.primary().head;
//         handle_key(&mut e, &Key::Character("f".into()));
//         handle_key(&mut e, &Key::Character("e".into())); // jump to first 'e'
//         assert!(e.selection.primary().cursor(e.document.rope().slice(..)) > start);
//         assert!(e.pending_find.is_none()); // consumed
//     }

//     #[test]
//     fn count_then_f_finds_nth() {
//         // "beebee": 'e' at char indices 1, 2, 4, 5. `fe` lands on the 1st; `2fe` on the 2nd.
//         let mut one = editor_with("beebee");
//         handle_key(&mut one, &Key::Character("f".into()));
//         handle_key(&mut one, &Key::Character("e".into()));
//         let p1 = one
//             .selection
//             .primary()
//             .cursor(one.document.rope().slice(..));

//         let mut two = editor_with("beebee");
//         handle_key(&mut two, &Key::Character("2".into()));
//         handle_key(&mut two, &Key::Character("f".into()));
//         handle_key(&mut two, &Key::Character("e".into()));
//         let p2 = two
//             .selection
//             .primary()
//             .cursor(two.document.rope().slice(..));

//         assert_eq!(p1, 1); // first 'e'
//         assert_eq!(p2, 2); // second 'e' — the count took effect
//         assert!(p2 > p1);
//         assert!(two.pending_find.is_none()); // find target consumed
//         assert_eq!(two.count, None); // count consumed
//     }

//     #[test]
//     fn w_extends_in_select_mode() {
//         let mut e = create_editor();
//         e.enter_select();
//         let anchor = e.selection.primary().anchor;
//         handle_key(&mut e, &Key::Character("w".to_string()));
//         let r = e.selection.primary();
//         assert_eq!(r.anchor, anchor); // anchor stays put
//         assert!(r.head > anchor); // head extends over the word
//     }

//     #[test]
//     fn l_extends_selection_in_select_mode() {
//         let mut e = create_editor();
//         e.enter_select();
//         let anchor = e.selection.primary().anchor;
//         handle_key(&mut e, &Key::Character("l".to_string()));
//         let r = e.selection.primary();
//         assert_eq!(r.anchor, anchor);
//         assert!(r.head > anchor);
//     }

//     #[test]
//     fn w_moves_selection_forward() {
//         let mut e = create_editor();
//         let before = e.selection.primary().head;
//         handle_key(&mut e, &Key::Character("w".to_string()));
//         assert!(e.selection.primary().head > before);
//     }
//     #[test]
//     fn e_moves_selection_forward() {
//         let mut e = create_editor();
//         let before = e.selection.primary().head;
//         handle_key(&mut e, &Key::Character("e".to_string()));
//         assert!(e.selection.primary().head > before);
//     }
//     #[test]
//     fn b_moves_selection_backward() {
//         let mut e = create_editor();
//         handle_key(&mut e, &Key::Character("w".to_string()));
//         handle_key(&mut e, &Key::Character("w".to_string()));
//         let mid = e.selection.primary().head;
//         handle_key(&mut e, &Key::Character("b".to_string()));
//         assert!(e.selection.primary().head < mid);
//     }

//     #[test]
//     fn i_enters_insert_mode_in_normal_mode() {
//         let mut editor = create_editor();
//         handle_key(&mut editor, &Key::Character("i".to_string()));
//         assert_eq!(editor.mode, Mode::Insert);
//     }

//     #[test]
//     fn i_enters_insert_mode_in_select_mode() {
//         let mut editor = create_editor();
//         editor.enter_select();
//         handle_key(&mut editor, &Key::Character("i".to_string()));
//         assert_eq!(editor.mode, Mode::Insert);
//     }

//     #[test]
//     fn esc_enters_normal_mode_in_insert_mode() {
//         let mut editor = create_editor();
//         editor.enter_insert();
//         handle_key(&mut editor, &Key::Named(NamedKey::Escape));
//         assert_eq!(editor.mode, Mode::Normal);
//     }

//     #[test]
//     fn v_enters_select_mode_in_normal_mode() {
//         let mut editor = create_editor();
//         handle_key(&mut editor, &Key::Character("v".to_string()));
//         assert_eq!(editor.mode, Mode::Select);
//     }

//     #[test]
//     fn v_enters_normal_mode_in_select_mode() {
//         let mut editor = create_editor();
//         editor.enter_select();
//         handle_key(&mut editor, &Key::Character("v".to_string()));
//         assert_eq!(editor.mode, Mode::Normal);
//     }

//     #[test]
//     fn digits_accumulate_into_count() {
//         let mut e = create_editor();
//         handle_key(&mut e, &Key::Character("1".into()));
//         handle_key(&mut e, &Key::Character("2".into()));
//         assert_eq!(e.count, Some(12));
//     }

//     #[test]
//     fn count_then_motion_moves_n_lines_and_clears() {
//         let mut e = create_editor();
//         let line0 = e.document.line_idx(e.selection.primary().head);
//         handle_key(&mut e, &Key::Character("3".into()));
//         handle_key(&mut e, &Key::Character("j".into()));
//         let line_after = e.document.line_idx(e.selection.primary().head);
//         assert_eq!(line_after - line0, 3);
//         assert_eq!(e.count, None); // consumed
//     }

//     #[test]
//     fn leading_zero_is_not_a_count() {
//         let mut e = create_editor();
//         handle_key(&mut e, &Key::Character("0".into()));
//         assert_eq!(e.count, None); // fell through to command dispatch (0 unbound -> no-op)
//     }

//     #[test]
//     fn gg_goes_to_file_start() {
//         let mut e = create_editor();
//         handle_key(&mut e, &Key::Character("j".into())); // move off 0
//         handle_key(&mut e, &Key::Character("g".into()));
//         handle_key(&mut e, &Key::Character("g".into()));
//         assert_eq!(e.selection.primary().cursor(e.document.rope().slice(..)), 0);
//         assert!(!e.pending_goto);
//     }
// }
