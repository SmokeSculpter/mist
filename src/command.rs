use std::sync::LazyLock;

use crate::{
    editor::{Editor, PendingFind},
    keymap::KeyMap,
    movement::{Direction, Movement},
};
use floem::{imbl::HashMap, prelude::Key};

type OnKeyCallBack = Box<dyn FnOnce(&mut Context, &Key)>;

pub struct Context {
    pub editor: Editor,
    pub register: Vec<String>,
    pub key_buffer: Option<String>,
    pub on_next_key: Option<OnKeyCallBack>,
    pub count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub name: &'static str,
    pub fun: fn(&mut Context),
}

impl Context {
    pub fn new(editor: Editor) -> Self {
        Self {
            editor,
            register: Vec::new(),
            key_buffer: None,
            on_next_key: None,
            count: None,
        }
    }

    pub fn get_key_buffer(&self) -> Option<String> {
        self.key_buffer.clone()
    }

    // Append number to count.
    // so 1 then 2 results in 12
    pub fn append_count_digit(&mut self, n: usize) {
        self.count = Some(self.count.unwrap_or(0) * 10 + n);
    }

    // Return count if Some or 1 if None
    pub fn take_count(&mut self) -> usize {
        self.count.take().unwrap_or(1)
    }
}

pub static STATIC_COMMAND_MAP: LazyLock<HashMap<&'static str, &'static Command>> =
    LazyLock::new(|| STATIC_COMMANDS.iter().map(|c| (c.name, c)).collect());

static STATIC_COMMANDS: &[Command] = &[
    Command {
        name: "move_cursor_left",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor
                .move_h(Direction::Backward, count, Movement::Move);
        },
    },
    Command {
        name: "extend_cursor_left",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor
                .move_h(Direction::Backward, count, Movement::Extend);
        },
    },
    Command {
        name: "move_cursor_right",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.move_h(Direction::Forward, count, Movement::Move);
        },
    },
    Command {
        name: "extend_cursor_right",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor
                .move_h(Direction::Forward, count, Movement::Extend);
        },
    },
    Command {
        name: "move_cursor_up",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor
                .move_v(Direction::Backward, count, Movement::Move);
        },
    },
    Command {
        name: "extend_cursor_up",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor
                .move_v(Direction::Backward, count, Movement::Extend);
        },
    },
    Command {
        name: "move_cursor_down",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.move_v(Direction::Forward, count, Movement::Move);
        },
    },
    Command {
        name: "extend_cursor_down",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor
                .move_v(Direction::Forward, count, Movement::Extend);
        },
    },
    Command {
        name: "enter_insert_mode",
        fun: |ctx: &mut Context| {
            ctx.editor.enter_insert();
        },
    },
    Command {
        name: "enter_insert_append",
        fun: |ctx: &mut Context| {
            ctx.editor.enter_insert_append();
        },
    },
    Command {
        name: "insert_at_line_start",
        fun: |ctx: &mut Context| {
            ctx.editor.insert_at_line_start();
        },
    },
    Command {
        name: "insert_at_line_end",
        fun: |ctx: &mut Context| {
            ctx.editor.insert_at_line_end();
        },
    },
    Command {
        name: "open_below",
        fun: |ctx: &mut Context| {
            ctx.editor.open_below();
        },
    },
    Command {
        name: "open_above",
        fun: |ctx: &mut Context| {
            ctx.editor.open_above();
        },
    },
    Command {
        name: "enter_select",
        fun: |ctx: &mut Context| {
            ctx.editor.enter_select();
        },
    },
    Command {
        name: "enter_normal",
        fun: |ctx: &mut Context| {
            ctx.editor.enter_normal();
        },
    },
    Command {
        name: "move_next_word_start",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.move_next_word_start(count);
        },
    },
    Command {
        name: "extend_next_word_start",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.extend_next_word_start(count);
        },
    },
    Command {
        name: "move_next_word_end",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.move_next_word_end(count);
        },
    },
    Command {
        name: "extend_next_word_end",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.extend_next_word_end(count);
        },
    },
    Command {
        name: "move_next_long_word_end",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.move_next_long_word_end(count);
        },
    },
    Command {
        name: "extend_next_long_word_end",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.extend_next_long_word_end(count);
        },
    },
    Command {
        name: "move_next_long_word_start",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.move_next_long_word_start(count);
        },
    },
    Command {
        name: "extend_next_long_word_start",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.extend_next_long_word_start(count);
        },
    },
    Command {
        name: "move_prev_word_start",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.move_prev_word_start(count);
        },
    },
    Command {
        name: "extend_prev_word_start",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.extend_prev_word_start(count);
        },
    },
    Command {
        name: "move_prev_long_word_start",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.move_prev_long_word_start(count);
        },
    },
    Command {
        name: "extend_prev_long_word_start",
        fun: |ctx: &mut Context| {
            let count = ctx.take_count();
            ctx.editor.extend_prev_long_word_start(count);
        },
    },
    Command {
        name: "delete_selections",
        fun: |ctx: &mut Context| {
            ctx.editor.delete_selections();
        },
    },
    Command {
        name: "change_selections",
        fun: |ctx: &mut Context| {
            ctx.editor.change_selections();
        },
    },
    Command {
        name: "yank",
        fun: |ctx: &mut Context| {
            ctx.editor.yank();
        },
    },
    Command {
        name: "paste_after_cursor",
        fun: |ctx: &mut Context| {
            ctx.editor.paste(true);
        },
    },
    Command {
        name: "paste_before_cursor",
        fun: |ctx: &mut Context| {
            ctx.editor.paste(false);
        },
    },
    Command {
        name: "undo",
        fun: |ctx: &mut Context| {
            ctx.editor.undo();
        },
    },
    Command {
        name: "redo",
        fun: |ctx: &mut Context| {
            ctx.editor.redo();
        },
    },
    Command {
        name: "move_on_next_char_forward",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| match key {
                Key::Character(ch) => {
                    if let Some(c) = ch.chars().next() {
                        let count = ctx.editor.take_count();
                        let pending_find = PendingFind {
                            count: count,
                            dir: Direction::Forward,
                            inclusive: true,
                            extend: false,
                        };
                        ctx.editor.find_char(&pending_find, c);
                    };
                }
                _ => {}
            }));
        },
    },
    Command {
        name: "extend_on_next_char_forward",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| match key {
                Key::Character(ch) => {
                    if let Some(c) = ch.chars().next() {
                        let count = ctx.editor.take_count();
                        let pending_find = PendingFind {
                            count: count,
                            dir: Direction::Forward,
                            inclusive: true,
                            extend: true,
                        };
                        ctx.editor.find_char(&pending_find, c);
                    };
                }
                _ => {}
            }));
        },
    },
    Command {
        name: "move_on_next_char_backward",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| match key {
                Key::Character(ch) => {
                    if let Some(c) = ch.chars().next() {
                        let count = ctx.editor.take_count();
                        let pending_find = PendingFind {
                            count: count,
                            dir: Direction::Backward,
                            inclusive: true,
                            extend: false,
                        };
                        ctx.editor.find_char(&pending_find, c);
                    };
                }
                _ => {}
            }));
        },
    },
    Command {
        name: "extend_on_next_char_backward",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| match key {
                Key::Character(ch) => {
                    if let Some(c) = ch.chars().next() {
                        let count = ctx.editor.take_count();
                        let pending_find = PendingFind {
                            count: count,
                            dir: Direction::Backward,
                            inclusive: true,
                            extend: true,
                        };
                        ctx.editor.find_char(&pending_find, c);
                    };
                }
                _ => {}
            }));
        },
    },
    Command {
        name: "move_before_char_forward",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| match key {
                Key::Character(ch) => {
                    if let Some(c) = ch.chars().next() {
                        let count = ctx.editor.take_count();
                        let pending_find = PendingFind {
                            count: count,
                            dir: Direction::Forward,
                            inclusive: false,
                            extend: false,
                        };
                        ctx.editor.find_char(&pending_find, c);
                    };
                }
                _ => {}
            }));
        },
    },
    Command {
        name: "extend_before_char_forward",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| match key {
                Key::Character(ch) => {
                    if let Some(c) = ch.chars().next() {
                        let count = ctx.editor.take_count();
                        let pending_find = PendingFind {
                            count: count,
                            dir: Direction::Forward,
                            inclusive: false,
                            extend: true,
                        };
                        ctx.editor.find_char(&pending_find, c);
                    };
                }
                _ => {}
            }));
        },
    },
    Command {
        name: "move_before_char_backward",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| match key {
                Key::Character(ch) => {
                    if let Some(c) = ch.chars().next() {
                        let count = ctx.editor.take_count();
                        let pending_find = PendingFind {
                            count: count,
                            dir: Direction::Backward,
                            inclusive: false,
                            extend: false,
                        };
                        ctx.editor.find_char(&pending_find, c);
                    };
                }
                _ => {}
            }));
        },
    },
    Command {
        name: "extend_before_char_backward",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| match key {
                Key::Character(ch) => {
                    if let Some(c) = ch.chars().next() {
                        let count = ctx.editor.take_count();
                        let pending_find = PendingFind {
                            count: count,
                            dir: Direction::Backward,
                            inclusive: false,
                            extend: true,
                        };
                        ctx.editor.find_char(&pending_find, c);
                    };
                }
                _ => {}
            }));
        },
    },
    Command {
        name: "goto_pending",
        fun: |ctx: &mut Context| {
            ctx.on_next_key = Some(Box::new(|ctx: &mut Context, key: &Key| {
                match key {
                    Key::Character(ch) => match ch.as_str() {
                        "h" => ctx.editor.goto_line_start(false),
                        "l" => ctx.editor.goto_file_end(false),
                        _ => {}
                    },
                    _ => {}
                }
                ctx.on_next_key = None;
            }))
        },
    },
    Command {
        name: "goto_line_start",
        fun: |ctx: &mut Context| {
            ctx.editor.goto_line_start(false);
        },
    },
    Command {
        name: "goto_line_start_extend",
        fun: |ctx: &mut Context| {
            ctx.editor.goto_line_start(true);
        },
    },
    Command {
        name: "goto_line_end",
        fun: |ctx: &mut Context| {
            ctx.editor.goto_file_end(false);
        },
    },
    Command {
        name: "goto_line_end_extend",
        fun: |ctx: &mut Context| {
            ctx.editor.goto_file_end(true);
        },
    },
    Command {
        name: "goto_file_start",
        fun: |ctx: &mut Context| {
            ctx.editor.goto_file_start(false);
        },
    },
    Command {
        name: "goto_line_start_extend",
        fun: |ctx: &mut Context| {
            ctx.editor.goto_file_start(true);
        },
    },
    Command {
        name: "goto_file_end",
        fun: |ctx: &mut Context| {
            ctx.editor.goto_file_end(false);
        },
    },
    Command {
        name: "goto_file_end_extend",
        fun: |ctx: &mut Context| {
            ctx.editor.goto_file_end(true);
        },
    },
    Command {
        name: "insert_new_line",
        fun: |ctx: &mut Context| {
            ctx.editor.insert_new_line();
        },
    },
    Command {
        name: "insert_tab",
        fun: |ctx: &mut Context| {
            ctx.editor.insert_tab();
        },
    },
];
