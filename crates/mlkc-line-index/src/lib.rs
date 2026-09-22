//! The index of the lines of a text.
//!
//! A diagnostic points at a byte range, and a person — and the language server protocol —
//! read a position as a line and a column. [`LineIndex`] is the mapping between the two:
//! it remembers where every line starts and ends, and answers in both directions.
//!
//! The columns are counted in bytes, which is what a byte offset can be turned into without
//! reading the text again. A consumer that speaks a different unit — the language server
//! protocol counts UTF-16 code units — converts a column of a line it already has,
//! which is a scan of that one line and not of the file.

use mlkc_text_size::{TextLen, TextRange, TextSize};

/// The offset of the first byte of every line of a text, and one past the last byte of it.
///
/// A line never holds its terminator: `"\r\n"` and `"\n"` belong to the line break,
/// so the text of a line is exactly what a reader sees.
#[derive(Debug)]
pub struct LineIndex {
    /// The offset of the first byte of each line, ascending; the first is always `0`.
    starts: Vec<TextSize>,
    /// The offset one past the last byte of each line, terminators excluded.
    ends: Vec<TextSize>,
    /// The length of the text.
    len: TextSize,
}

/// A line and a column, both counted from zero.
///
/// The column is a count of bytes from the start of the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl LineIndex {
    /// Indexes the lines of `text`.
    ///
    /// A text has at least one line, and it has one more when it ends with a line break:
    /// the line a cursor sits on after pressing Enter at the end of a file is a real line.
    pub fn new(text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut starts = vec![TextSize::from(0)];
        let mut ends = Vec::new();
        let mut line_start = TextSize::from(0);

        for (index, &byte) in bytes.iter().enumerate() {
            if byte != b'\n' {
                continue;
            }

            let break_start = TextSize::from(index as u32);
            let break_end = TextSize::from(index as u32 + 1);

            // `\r\n` is one line break, and the `\r` is part of it.
            let end = if break_start > line_start && bytes[index - 1] == b'\r' {
                break_start - TextSize::from(1)
            } else {
                break_start
            };

            ends.push(end);
            starts.push(break_end);
            line_start = break_end;
        }

        ends.push(text.text_len());

        Self {
            starts,
            ends,
            len: text.text_len(),
        }
    }

    /// The length of the text the index was built from.
    pub fn len(&self) -> TextSize {
        self.len
    }

    /// Whether the text holds nothing.
    pub fn is_empty(&self) -> bool {
        self.len == TextSize::from(0)
    }

    /// How many lines the text has.
    pub fn line_count(&self) -> u32 {
        self.starts.len() as u32
    }

    /// The line and the column `offset` is at.
    ///
    /// An offset past the end of the text is read as the end of the text,
    /// which is what a diagnostic that points past the last token means.
    pub fn line_col(&self, offset: TextSize) -> LineCol {
        let offset = offset.min(self.len);

        // The first start is `0` and no start is greater than the offset of the end,
        // so the line is always found.
        let line = self.starts.partition_point(|&start| start <= offset) - 1;

        LineCol {
            line: line as u32,
            col: u32::from(offset - self.starts[line]),
        }
    }

    /// The offset a line and a column point at, if the line exists.
    ///
    /// A column past the end of its line is read as the end of that line:
    /// a request for a position at the end of a file is a request, not a mistake.
    pub fn offset(&self, line_col: LineCol) -> Option<TextSize> {
        let index = line_col.line as usize;
        let start = *self.starts.get(index)?;
        let end = *self.ends.get(index)?;
        let col = line_col.col.min(u32::from(end - start));

        Some(start + TextSize::from(col))
    }

    /// The range of a line, without its terminator.
    pub fn line_range(&self, line: u32) -> Option<TextRange> {
        let index = line as usize;
        let start = *self.starts.get(index)?;
        let end = *self.ends.get(index)?;

        Some(TextRange::new(start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: u32, col: u32) -> LineCol {
        LineCol { line, col }
    }

    #[test]
    fn an_empty_text_is_one_empty_line() {
        let index = LineIndex::new("");

        assert_eq!(index.line_count(), 1);
        assert_eq!(
            index.line_range(0),
            Some(TextRange::empty(TextSize::from(0)))
        );
        assert_eq!(index.line_col(TextSize::from(0)), at(0, 0));
        assert_eq!(index.offset(at(0, 0)), Some(TextSize::from(0)));
        assert!(index.is_empty());
    }

    #[test]
    fn the_lines_of_a_text_are_the_ones_a_reader_sees() {
        let text = "one\ntwo\nthree";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 3);
        assert_eq!(&text[index.line_range(0).unwrap()], "one");
        assert_eq!(&text[index.line_range(1).unwrap()], "two");
        assert_eq!(&text[index.line_range(2).unwrap()], "three");
        assert_eq!(index.line_range(3), None);
    }

    #[test]
    fn a_text_that_ends_with_a_break_has_a_line_after_it() {
        let text = "one\n";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 2);
        assert_eq!(&text[index.line_range(1).unwrap()], "");
        assert_eq!(index.line_col(text.text_len()), at(1, 0));
    }

    #[test]
    fn a_line_break_of_two_bytes_belongs_to_the_break() {
        let text = "one\r\ntwo";
        let index = LineIndex::new(text);

        assert_eq!(&text[index.line_range(0).unwrap()], "one");
        assert_eq!(&text[index.line_range(1).unwrap()], "two");
    }

    #[test]
    fn offsets_and_positions_agree() {
        let text = "one\ntwo\n";
        let index = LineIndex::new(text);

        for (offset, expected) in [
            (0, at(0, 0)),
            (2, at(0, 2)),
            (3, at(0, 3)),
            (4, at(1, 0)),
            (7, at(1, 3)),
            (8, at(2, 0)),
        ] {
            let offset = TextSize::from(offset);

            assert_eq!(index.line_col(offset), expected, "at {offset:?}");
            assert_eq!(index.offset(expected), Some(offset), "at {expected:?}");
        }
    }

    #[test]
    fn an_offset_past_the_text_is_the_end_of_the_text() {
        let index = LineIndex::new("one");

        assert_eq!(index.line_col(TextSize::from(42)), at(0, 3));
    }

    #[test]
    fn a_column_past_the_line_is_the_end_of_the_line() {
        let index = LineIndex::new("one\ntwo");

        assert_eq!(index.offset(at(0, 42)), Some(TextSize::from(3)));
        assert_eq!(index.offset(at(2, 0)), None, "there is no third line");
    }
}
