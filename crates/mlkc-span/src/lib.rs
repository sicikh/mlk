pub use mlkc_text_size::{TextLen, TextRange, TextSize};

use mlkc_vfs::FileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub range: TextRange,
}

impl Span {
    pub const fn new(file: FileId, range: TextRange) -> Self {
        Span { file, range }
    }

    pub fn dummy() -> Self {
        Span {
            file: FileId::DUMMY,
            range: TextRange::new(TextSize::from(0), TextSize::from(0)),
        }
    }

    pub fn is_dummy(&self) -> bool {
        self == &Span::dummy()
    }

    pub const fn start(&self) -> TextSize {
        self.range.start()
    }

    pub const fn end(&self) -> TextSize {
        self.range.end()
    }

    pub fn len(&self) -> TextSize {
        self.range.len()
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }
}
