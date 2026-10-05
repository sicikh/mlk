//! The place a thing was written: a file, and a range of its text.
//!
//! The compiler reads and writes text in bytes,
//! so everything that points back at the source --- a token, the label of a diagnostic,
//! an item of the HIR --- points with a [`Span`].
//! A span is a plain value that is copied and stored:
//! it outlives the tree it was read from, which is what lets a report be built and kept
//! after the parse that raised it is gone.
//!
//! [`Span::dummy`] stands for what has no place in any file,
//! such as a node the compiler made up while lowering.

pub use mlkc_text_size::{TextLen, TextRange, TextSize};
pub use mlkc_vfs::FileId;

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
