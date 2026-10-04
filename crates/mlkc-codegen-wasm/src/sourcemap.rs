//! The source map of a module ([ADR-0025][adr-0025]).
//!
//! A browser reads a source map rather than DWARF: V8 parses the `sourceMappingURL` custom
//! section of a module and hands the URL to DevTools, which loads and parses the map itself.
//! The map is written here as a `data:` URL embedded in the module, so a module carries the
//! text of every file its bodies were read from --- `sourcesContent` --- and the browser shows
//! them without a host serving anything ([ADR-0025][adr-0025]).
//!
//! The convention of a WASM source map is the one Emscripten writes and V8 reads: every
//! segment is on generated line zero, and the generated column is the module-relative byte
//! offset of the instruction --- the code-section offset of [`Layout`] plus the offset of the
//! contents of the code section in the module. The original line and column count from zero,
//! which is where the [`LineIndex`](mlkc_line_index::LineIndex) of a file counts from too.
//!
//! [adr-0025]: ../../docs/adr/0025-debug-information-formats.md

use std::{collections::BTreeMap, fmt::Write as _};

use mlkc_span::FileId;

use crate::{
    dwarf::{Layout, SourceFile, Sources},
    emit::FuncArtifact,
};

/// The name of the custom section a browser reads the map from.
const SECTION: &str = "sourceMappingURL";

/// The source map of a module, as the custom section a browser reads.
///
/// `code_payload` is the module-relative offset of the contents of the code section, which is
/// what the addresses of the layout are relative to; `None` when the bodies carry no origin,
/// since a map of nothing points nowhere.
pub(crate) fn section(
    functions: &[FuncArtifact],
    sources: &Sources,
    layout: &Layout,
    code_payload: u32,
) -> Option<(&'static str, Vec<u8>)> {
    let mut files: BTreeMap<FileId, usize> = BTreeMap::new();
    let mut texts: Vec<&SourceFile> = Vec::new();
    let mut mappings = String::new();
    let mut previous = Segment::default();

    for (index, artifact) in functions.iter().enumerate() {
        let at = layout.functions[index];

        for origin in &artifact.origins {
            if origin.span.is_dummy() {
                continue;
            }

            let source = sources
                .get(origin.span.file)
                .expect("the driver hands the text of every file a body names");
            let file = match files.get(&origin.span.file) {
                Some(file) => *file,
                None => {
                    let file = texts.len();

                    files.insert(origin.span.file, file);
                    texts.push(source);

                    file
                },
            };
            let position = source.lines.line_col(origin.span.start());
            let segment = Segment {
                address: u64::from(code_payload) + u64::from(at.content) + u64::from(origin.offset),
                file: file as u64,
                line: u64::from(position.line),
                column: u64::from(position.col),
            };

            if !mappings.is_empty() {
                mappings.push(',');
            }

            segment.write(&mut mappings, &mut previous);
        }
    }

    if texts.is_empty() {
        return None;
    }

    let mut json = String::from("{\"version\":3,\"sources\":[");

    for (index, file) in texts.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }

        string(&mut json, &file.path);
    }

    json.push_str("],\"sourcesContent\":[");

    for (index, file) in texts.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }

        string(&mut json, &file.text);
    }

    json.push_str("],\"names\":[],\"mappings\":\"");
    json.push_str(&mappings);
    json.push_str("\"}");

    // The map is a `data:` URL, so the module carries it itself; the section data is the URL
    // as a length-prefixed string, which is what V8 reads it as.
    let mut uri = String::from("data:application/json;charset=utf-8;base64,");

    base64(json.as_bytes(), &mut uri);

    let mut data = leb(uri.len() as u32);
    data.extend_from_slice(uri.as_bytes());

    Some((SECTION, data))
}

/// One segment of the map: where an instruction is, and what it was read from.
#[derive(Default, Clone, Copy)]
struct Segment {
    /// The module-relative byte offset of the instruction.
    address: u64,
    /// The place of its file in the map.
    file: u64,
    /// The line of the file, counting from zero.
    line: u64,
    /// The column of the line, counting from zero.
    column: u64,
}

impl Segment {
    /// Writes the segment relative to `previous`, which it becomes.
    ///
    /// A segment of a source map is a delta from the one before it; every segment of a WASM map
    /// is on the generated line zero, so the line is never written.
    fn write(&self, out: &mut String, previous: &mut Segment) {
        vlq(self.address, previous.address, out);
        vlq(self.file, previous.file, out);
        vlq(self.line, previous.line, out);
        vlq(self.column, previous.column, out);

        *previous = *self;
    }
}

/// Writes one number of a segment as a base64 VLQ delta, and leaves `previous` as it is.
fn vlq(value: u64, previous: u64, out: &mut String) {
    let delta = value as i64 - previous as i64;
    // The sign goes into the lowest bit, which is what zigzag encoding is.
    let mut rest = ((delta << 1) ^ (delta >> 63)) as u64;

    loop {
        let mut digit = (rest & 0x1F) as u8;

        rest >>= 5;

        if rest != 0 {
            digit |= 0x20;
        }

        out.push(BASE64[usize::from(digit)] as char);

        if rest == 0 {
            return;
        }
    }
}

/// The alphabet of base64, which is also the alphabet of the VLQ digits of a source map.
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Writes `bytes` as base64, padded as the standard says.
fn base64(bytes: &[u8], out: &mut String) {
    for chunk in bytes.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = chunk.get(1).copied().map_or(0, u32::from);
        let third = chunk.get(2).copied().map_or(0, u32::from);
        let bits = (first << 16) | (second << 8) | third;

        out.push(BASE64[((bits >> 18) & 0x3F) as usize] as char);
        out.push(BASE64[((bits >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            out.push(BASE64[((bits >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(BASE64[(bits & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
}

/// Writes a string as a JSON string, escaping what JSON escapes.
fn string(out: &mut String, text: &str) {
    out.push('"');

    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character < ' ' => {
                write!(out, "\\u{:04x}", u32::from(character))
                    .expect("writing to a string to never fail");
            },
            character => out.push(character),
        }
    }

    out.push('"');
}

/// A `u32` as an unsigned LEB128, which is how a WASM length is written.
fn leb(mut value: u32) -> Vec<u8> {
    let mut bytes = Vec::new();

    loop {
        let mut byte = (value & 0x7F) as u8;

        value >>= 7;

        if value != 0 {
            byte |= 0x80;
        }

        bytes.push(byte);

        if value == 0 {
            return bytes;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_is_a_base64_vlq_delta() {
        // The values Emscripten writes for the first addresses of its test fixture: the address
        // 110, then the step to 117, which is seven bytes further.
        let mut out = String::new();

        vlq(110, 0, &mut out);
        vlq(117, 110, &mut out);

        assert_eq!(out, "8GO");

        // A line that does not move, then one that moves by three, which is the line of the
        // fixture: the wire counts from zero, and the delta is written.
        let mut out = String::new();

        vlq(0, 0, &mut out);
        vlq(3, 0, &mut out);
        vlq(0, 3, &mut out);

        assert_eq!(out, "AGF");
    }

    #[test]
    fn a_string_is_written_as_json() {
        let mut out = String::new();

        string(&mut out, "/main.mlk");

        assert_eq!(out, "\"/main.mlk\"");

        let mut out = String::new();

        string(&mut out, "a\"b\\c\nd");

        assert_eq!(out, "\"a\\\"b\\\\c\\nd\"");
    }

    #[test]
    fn bytes_are_written_as_base64() {
        let mut out = String::new();

        base64(b"", &mut out);
        assert_eq!(out, "");

        let mut out = String::new();

        base64(b"f", &mut out);
        assert_eq!(out, "Zg==");

        let mut out = String::new();

        base64(b"fo", &mut out);
        assert_eq!(out, "Zm8=");

        let mut out = String::new();

        base64(b"foo", &mut out);
        assert_eq!(out, "Zm9v");

        let mut out = String::new();

        base64(b"foobar", &mut out);
        assert_eq!(out, "Zm9vYmFy");
    }

    #[test]
    fn a_length_is_a_leb128() {
        assert_eq!(leb(0), [0x00]);
        assert_eq!(leb(47), [0x2F]);
        assert_eq!(leb(128), [0x80, 0x01]);
        assert_eq!(leb(624_485), [0xE5, 0x8E, 0x26]);
    }
}
