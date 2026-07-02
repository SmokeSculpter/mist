use crate::document::Document;
use crate::selection::Range;
use anyhow::Result;
use std::path::Path;

pub struct EditorState {
    pub document: Document,
    pub range: Range,
}

impl EditorState {
    pub fn new(path: &Path) -> Result<Self> {
        let range = Range::new();
        let document = Document::open(path, None)?;

        Ok(Self {
            document: document,
            range: range,
        })
    }
}
