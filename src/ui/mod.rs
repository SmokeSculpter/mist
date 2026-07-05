//! The UI layer: pure render geometry (`render`) + the floem view and input
//! wiring (`editor_view`). Kept separate from the editor model so the state layer
//! stays headless-testable.

pub mod editor_view;
pub mod render;
