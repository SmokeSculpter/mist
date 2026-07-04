use crate::document::Document;
use crate::selection::Selection;
use anyhow::Result;
use std::path::Path;

pub struct Editor {
    pub document: Document,
    pub selection: Selection,
}

impl Editor {
    pub fn new(path: &Path) -> Result<Self> {
        let selection = Selection::point(0);
        let document = Document::open(path, None)?;

        Ok(Self {
            document: document,
            selection,
        })
    }
}
