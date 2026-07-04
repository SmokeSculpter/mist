//! Editor modes. Which mode is active gates how `keymap::handle_key` interprets
//! a keypress and how the cursor is rendered (block in Normal/Select, thin bar in
//! Insert). Modeled on Helix's modal model.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Keys are commands; motions collapse the selection to a cursor.
    Normal,
    /// Keys insert text; Esc returns to Normal.
    Insert,
    /// Like Normal, but motions extend the selection instead of collapsing it.
    Select,
}
