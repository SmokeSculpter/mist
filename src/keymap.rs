use crate::{editor::Editor, mode::Mode, movement::Direction};
use floem::prelude::{Key, NamedKey};

pub fn handle_key(editor: &mut Editor, key: &Key) {
    match editor.mode {
        Mode::Normal => match key {
            Key::Character(ch) if ch == "h" => editor.move_h(Direction::Backward, 1),
            Key::Character(ch) if ch == "l" => editor.move_h(Direction::Forward, 1),
            Key::Character(ch) if ch == "j" => editor.move_v(Direction::Forward, 1),
            Key::Character(ch) if ch == "k" => editor.move_v(Direction::Backward, 1),
            Key::Character(ch) if ch == "i" => editor.enter_insert(),
            Key::Character(ch) if ch == "v" => editor.enter_select(),
            _ => {}
        },
        Mode::Insert => match key {
            Key::Named(NamedKey::Escape) => editor.enter_normal(),
            _ => {}
        },
        Mode::Select => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::{editor::Editor, keymap::handle_key, mode::Mode, movement::Direction};
    use floem::prelude::{Key, NamedKey};
    use std::path::Path;

    fn create_editor() -> Editor {
        let path = "./src/document.rs";
        Editor::new(Path::new(&path)).unwrap()
    }

    #[test]
    fn i_enters_insert_mode_in_normal_mode() {
        let mut editor = create_editor();
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
}
