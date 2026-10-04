//! The debug information of a compiled module ([ADR-0025][adr-0025]).
//!
//! A module assembled at the `Lines` level or above carries DWARF custom sections, and this is
//! what reads them back: the line program points at the file and the line a body was read from,
//! and every function is a subprogram over its body. What the tables cannot say alone is where a
//! body stands in the code section, so the addresses are checked against the bytes the assembler
//! wrote --- the same comparison `wasm-tools addr2line` makes, in the convention LLVM emits:
//! a code address is an offset from the contents of the code section, and a function begins at
//! its body content, after the length prefix.
//!
//! The same module carries a source map, the format a browser reads without an extension
//! ([ADR-0025][adr-0025]): it points at the same lines the line program does, counted in module
//! bytes rather than code-section offsets, and it carries the text of the file itself.
//!
//! [adr-0025]: ../../../docs/adr/0025-debug-information-formats.md

mod harness;

use std::{collections::BTreeMap, ops::Range};

use gimli::{
    AttributeValue, DebuggingInformationEntry, Dwarf, EndianSlice, LittleEndian,
    constants::{DW_AT_high_pc, DW_AT_low_pc, DW_AT_name, DW_TAG_subprogram},
};
use mlkc_codegen_wasm::DebugInfo;
use serde_json::Value;
use wasmparser::{Parser, Payload};

/// What the tables are read with: the bytes of the module, little endian as WASM writes them.
type Slice<'a> = EndianSlice<'a, LittleEndian>;

/// A program whose bodies stand on known lines: each function's literal is on a line of its own.
const SOURCE: &str =
    "//- /main.mlk\npub fun first(): Int =\n    1\n\npub fun second(): Int =\n    2\n";

/// The text of the file of the fixture: what a map of the module must carry.
const FILE: &str = "pub fun first(): Int =\n    1\n\npub fun second(): Int =\n    2\n";

/// A module carries one format of debug information and not two ([`DebugInfo`]).
#[test]
fn the_debug_information_is_one_format() {
    let bare = harness::project_with(SOURCE, DebugInfo::None);
    let sections = custom_sections(&bare.wasm.bytes);

    assert!(
        sections.contains_key("name"),
        "the name section to be a part of the module at every option",
    );
    assert!(
        !sections.contains_key("sourceMappingURL")
            && sections.keys().all(|name| !name.starts_with(".debug")),
        "a module without debug information to carry nothing else: {:?}",
        sections.keys().collect::<Vec<_>>(),
    );

    bare.validate();

    let mapped = harness::project_with(SOURCE, DebugInfo::SourceMap);
    let sections = custom_sections(&mapped.wasm.bytes);

    assert!(
        sections.contains_key("name"),
        "the name section to be a part of the module at every option",
    );
    assert!(
        sections.contains_key("sourceMappingURL"),
        "the map to be what the browser option carries: {:?}",
        sections.keys().collect::<Vec<_>>(),
    );
    assert!(
        sections.keys().all(|name| !name.starts_with(".debug")),
        "a module of a map to carry no DWARF: {:?}",
        sections.keys().collect::<Vec<_>>(),
    );

    mapped.validate();

    for debug in [DebugInfo::DwarfLines, DebugInfo::DwarfFull] {
        let compiled = harness::project_with(SOURCE, debug);
        let sections = custom_sections(&compiled.wasm.bytes);

        for name in [".debug_abbrev", ".debug_info", ".debug_line"] {
            assert!(
                sections.contains_key(name),
                "`{name}` to be written at this option: {:?}",
                sections.keys().collect::<Vec<_>>(),
            );
        }
        assert!(
            !sections.contains_key("sourceMappingURL"),
            "a module of tables to carry no map: {:?}",
            sections.keys().collect::<Vec<_>>(),
        );

        compiled.validate();
    }
}

/// Every function is a subprogram that covers the body it was emitted as.
#[test]
fn every_function_is_a_subprogram_over_its_body() {
    let compiled = harness::project_with(SOURCE, DebugInfo::DwarfLines);
    let (contents, bodies) = bodies(&compiled.wasm.bytes);
    let subprograms = subprograms(&compiled.wasm.bytes);

    assert_eq!(
        subprograms.len(),
        compiled.module.functions.len(),
        "one subprogram per function",
    );
    assert_eq!(bodies.len(), compiled.module.functions.len());

    for (function, (subprogram, body)) in compiled
        .module
        .functions
        .iter()
        .zip(subprograms.iter().zip(&bodies))
    {
        assert_eq!(subprogram.name, function.name);
        assert_eq!(
            subprogram.low_pc,
            (body.start - contents) as u64,
            "`{}` to begin where its body does",
            function.name,
        );
        assert_eq!(
            subprogram.high_pc,
            body.len() as u64,
            "`{}` to reach the end of its body",
            function.name,
        );
    }
}

/// The line program names the file and the line of the source a body was read from.
#[test]
fn the_line_program_names_the_source_of_a_body() {
    let compiled = harness::project_with(SOURCE, DebugInfo::DwarfLines);
    let (_, bodies) = bodies(&compiled.wasm.bytes);
    let rows = rows(&compiled.wasm.bytes);
    let subprograms = subprograms(&compiled.wasm.bytes);

    assert_eq!(subprograms.len(), bodies.len());

    for (index, function) in compiled.module.functions.iter().enumerate() {
        let subprogram = &subprograms[index];
        let start = subprogram.low_pc;
        let end = start + subprogram.high_pc;
        let within: Vec<&Row> = rows
            .iter()
            .filter(|row| row.address >= start && row.address < end)
            .collect();

        assert!(!within.is_empty(), "`{}` to have a line", function.name);
        assert!(
            within.iter().all(|row| row.file == "/main.mlk"),
            "every line of `{}` to name its file: {within:?}",
            function.name,
        );
        assert!(
            within.iter().all(|row| row.line >= 1),
            "every line of `{}` to be a line of the file: {within:?}",
            function.name,
        );
    }

    // The literal of each function stands on the line after the signature, and something of the
    // body points at it.
    for (name, line) in [("first", 2), ("second", 5)] {
        let index = compiled
            .module
            .functions
            .iter()
            .position(|function| function.name == name)
            .expect("the function to be compiled");
        let subprogram = &subprograms[index];

        assert!(
            rows.iter().any(|row| {
                row.address >= subprogram.low_pc
                    && row.address < subprogram.low_pc + subprogram.high_pc
                    && row.line == line
            }),
            "`{name}` to have a row at line {line}: {:?}",
            rows.iter()
                .filter(|row| row.address >= subprogram.low_pc)
                .collect::<Vec<_>>(),
        );
    }
}

/// The map is carried by the option of a browser, and it carries the source itself.
#[test]
fn the_source_map_is_what_a_browser_reads() {
    let map = map(&harness::project_with(SOURCE, DebugInfo::SourceMap)
        .wasm
        .bytes);

    assert_eq!(map["version"].as_u64(), Some(3));
    assert_eq!(strings(&map["sources"]), ["/main.mlk"]);
    assert_eq!(strings(&map["sourcesContent"]), [FILE]);
}

/// Every segment of the map is a row of the line program, in module bytes and from zero.
///
/// The two are two builds of one project: the bodies are the same bytes, and a custom section
/// is written after the code section, so an instruction stands at the same module offset in
/// both, and the rows of one are what the segments of the other are compared with.
#[test]
fn the_segments_of_the_map_are_the_rows_of_the_line_program() {
    let mapped = harness::project_with(SOURCE, DebugInfo::SourceMap);
    let tables = harness::project_with(SOURCE, DebugInfo::DwarfLines);
    let (contents, _) = bodies(&tables.wasm.bytes);
    let rows = rows(&tables.wasm.bytes);
    let map = map(&mapped.wasm.bytes);
    let sources = strings(&map["sources"]);
    let mappings = map["mappings"].as_str().expect("the mappings of a map");
    let segments = segments(mappings);

    assert_eq!(segments.len(), rows.len(), "one segment per row");

    for (segment, row) in segments.iter().zip(&rows) {
        assert_eq!(
            segment.address,
            (contents + row.address as usize) as i64,
            "a segment to stand where its row does, counted in module bytes",
        );
        assert_eq!(sources[segment.file as usize], row.file);
        // A map counts lines and columns from zero, and a line program from one.
        assert_eq!(segment.line + 1, i64::from(row.line));
        assert_eq!(segment.column + 1, i64::from(row.column));
    }
}

/// The segments of the map are sorted by the byte they stand at, which a reader assumes.
#[test]
fn the_segments_of_the_map_are_sorted() {
    let compiled = harness::project_with(SOURCE, DebugInfo::SourceMap);
    let map = map(&compiled.wasm.bytes);
    let mappings = map["mappings"].as_str().expect("the mappings of a map");
    let segments = segments(mappings);
    let mut previous = None;

    for segment in &segments {
        if let Some(previous) = previous {
            assert!(
                segment.address > previous,
                "the segments to be sorted by address: {previous} then {}",
                segment.address,
            );
        }

        previous = Some(segment.address);
    }
}

/// The custom sections of an emitted module, by name.
fn custom_sections(bytes: &[u8]) -> BTreeMap<&str, &[u8]> {
    let mut sections = BTreeMap::new();

    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::CustomSection(section) = payload.expect("the emitted module to parse") {
            sections.insert(section.name(), section.data());
        }
    }

    sections
}

/// The code section of an emitted module: the offset of its contents --- the first byte DWARF
/// counts addresses from --- and the range of every body inside it.
fn bodies(bytes: &[u8]) -> (usize, Vec<Range<usize>>) {
    let mut contents = 0;
    let mut bodies = Vec::new();

    for payload in Parser::new(0).parse_all(bytes) {
        match payload.expect("the emitted module to parse") {
            Payload::CodeSectionStart { range, .. } => contents = range.start,
            Payload::CodeSectionEntry(body) => bodies.push(body.range()),
            _ => {},
        }
    }

    (contents, bodies)
}

/// The DWARF of an emitted module, read from its custom sections.
fn dwarf<'a>(bytes: &'a [u8]) -> Dwarf<Slice<'a>> {
    let sections: BTreeMap<&'a str, &'a [u8]> = custom_sections(bytes);

    Dwarf::load(|id| -> Result<Slice<'a>, std::convert::Infallible> {
        Ok(EndianSlice::new(
            sections.get(id.name()).copied().unwrap_or(&[]),
            LittleEndian,
        ))
    })
    .expect("loading the tables of an emitted module to be infallible")
}

/// One row of the line program: where it is, and what it points at.
#[derive(Debug)]
struct Row {
    /// The code-section offset of the instruction.
    address: u64,
    /// The path of the file the instruction was read from.
    file: String,
    /// The line of the file, counting from one.
    line: u32,
    /// The column of the line, counting from one.
    column: u32,
}

/// The rows of the line program of an emitted module, in the order they were written.
fn rows(bytes: &[u8]) -> Vec<Row> {
    let dwarf = dwarf(bytes);
    let mut found = Vec::new();
    let mut units = dwarf.units();

    while let Some(header) = units.next().expect("the units to parse") {
        let unit = dwarf.unit(header).expect("a unit to parse");
        let program = unit
            .line_program
            .clone()
            .expect("a module assembled at this level to carry a line program");
        let mut rows = program.rows();

        while let Some((header, row)) = rows.next_row().expect("the rows to parse") {
            if row.end_sequence() {
                continue;
            }

            let file = row
                .file(header)
                .map(|file| {
                    dwarf
                        .attr_string(&unit, file.path_name())
                        .expect("the name of a file to read")
                        .to_string_lossy()
                        .into_owned()
                })
                .expect("every row to name a file");

            found.push(Row {
                address: row.address(),
                file,
                line: row.line().map_or(0, |line| line.get() as u32),
                column: match row.column() {
                    gimli::ColumnType::LeftEdge => 0,
                    gimli::ColumnType::Column(column) => column.get() as u32,
                },
            });
        }
    }

    found
}

/// One subprogram of the tables: a function, and where its body stands.
#[derive(Debug)]
struct Subprogram {
    /// The name of the function.
    name: String,
    /// The code-section offset where the body begins.
    low_pc: u64,
    /// How far the body reaches from its beginning.
    high_pc: u64,
}

/// The subprograms of an emitted module, in the order they were written.
fn subprograms(bytes: &[u8]) -> Vec<Subprogram> {
    let dwarf = dwarf(bytes);
    let mut found = Vec::new();
    let mut units = dwarf.units();

    while let Some(header) = units.next().expect("the units to parse") {
        let unit = dwarf.unit(header).expect("a unit to parse");
        let mut entries = unit.entries();

        while let Some((_, entry)) = entries.next_dfs().expect("the entries to parse") {
            if entry.tag() != DW_TAG_subprogram {
                continue;
            }

            let name = attribute(entry, DW_AT_name)
                .map(|value| {
                    dwarf
                        .attr_string(&unit, value)
                        .expect("the name of a function to read")
                        .to_string_lossy()
                        .into_owned()
                })
                .expect("every subprogram to have a name");

            found.push(Subprogram {
                name,
                low_pc: number(entry, DW_AT_low_pc).expect("a subprogram to have a low address"),
                high_pc: number(entry, DW_AT_high_pc).expect("a subprogram to have a length"),
            });
        }
    }

    found
}

/// The value of one attribute of an entry, if it carries it.
fn attribute<'a>(
    entry: &DebuggingInformationEntry<Slice<'a>>,
    name: gimli::constants::DwAt,
) -> Option<AttributeValue<Slice<'a>>> {
    let mut attributes = entry.attrs();

    while let Some(attribute) = attributes.next().expect("the attributes to parse") {
        if attribute.name() == name {
            return Some(attribute.value());
        }
    }

    None
}

/// The number one attribute of an entry carries.
fn number(
    entry: &DebuggingInformationEntry<Slice<'_>>,
    name: gimli::constants::DwAt,
) -> Option<u64> {
    match attribute(entry, name)? {
        AttributeValue::Addr(address) => Some(address),
        AttributeValue::Udata(value) => Some(value),
        other => panic!("`{name:?}` to be a number, and it is {other:?}"),
    }
}

/// The URL of the source map of an emitted module, read from its custom section.
fn map_url(bytes: &[u8]) -> Option<&str> {
    for payload in Parser::new(0).parse_all(bytes) {
        let Payload::CustomSection(section) = payload.expect("the emitted module to parse") else {
            continue;
        };

        if section.name() != "sourceMappingURL" {
            continue;
        }

        let data = section.data();
        let (length, prefix) = leb(data);
        let url = &data[prefix..prefix + length];

        return Some(std::str::from_utf8(url).expect("the URL of a map to be text"));
    }

    None
}

/// The map of an emitted module, decoded from the `data:` URL of its custom section.
fn map(bytes: &[u8]) -> Value {
    let url = map_url(bytes).expect("a module at this level to carry a source map");
    let encoded = url
        .strip_prefix("data:application/json;charset=utf-8;base64,")
        .expect("a map to be a `data:` URL of JSON");

    serde_json::from_slice(&base64(encoded)).expect("a map to be JSON")
}

/// The strings of a JSON array of the map, which holds strings only.
fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("an array of the map")
        .iter()
        .map(|item| item.as_str().expect("a string of the map").to_owned())
        .collect()
}

/// The number a LEB128 carries, and how many bytes it takes.
fn leb(bytes: &[u8]) -> (usize, usize) {
    let mut value = 0usize;
    let mut shift = 0;

    for (index, byte) in bytes.iter().enumerate() {
        value |= usize::from(byte & 0x7F) << shift;

        if byte & 0x80 == 0 {
            return (value, index + 1);
        }

        shift += 7;
    }

    panic!("a LEB128 to end inside the bytes it is read from");
}

/// The alphabet of base64, which is also the alphabet of the VLQ digits of a map.
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The bytes a base64 string carries, which is what a `data:` URL of a map holds.
fn base64(text: &str) -> Vec<u8> {
    let mut bits = 0u32;
    let mut count = 0u32;
    let mut bytes = Vec::new();

    for character in text.bytes() {
        if character == b'=' {
            continue;
        }

        let digit = BASE64
            .iter()
            .position(|it| *it == character)
            .unwrap_or_else(|| panic!("`{}` is not base64", char::from(character)))
            as u32;

        bits = (bits << 6) | digit;
        count += 6;

        if count >= 8 {
            count -= 8;
            bytes.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }

    bytes
}

/// One segment of a map: where an instruction stands, and what it was read from.
#[derive(Debug, Clone, Copy, Default)]
struct Segment {
    /// The module byte offset of the instruction.
    address: i64,
    /// The place of its file in the map.
    file: i64,
    /// The line of the file, counting from zero.
    line: i64,
    /// The column of the line, counting from zero.
    column: i64,
}

/// The segments of a `mappings` string, in the order they were written.
fn segments(mappings: &str) -> Vec<Segment> {
    let mut found = Vec::new();
    let mut fields = [0i64; 4];
    let mut values: Vec<i64> = Vec::new();
    let mut value = 0u64;
    let mut shift = 0u32;

    for character in mappings.bytes() {
        if character == b',' {
            push(&mut found, &mut fields, &mut values);
            continue;
        }

        let digit = BASE64
            .iter()
            .position(|it| *it == character)
            .unwrap_or_else(|| panic!("`{}` is not a character of a map", char::from(character)))
            as u64;

        value |= (digit & 0x1F) << shift;
        shift += 5;

        if digit & 0x20 == 0 {
            // The lowest bit is the sign, which is what zigzag encoding is.
            let magnitude = (value >> 1) as i64;

            values.push(if value & 1 == 0 {
                magnitude
            } else {
                -magnitude
            });
            value = 0;
            shift = 0;
        }
    }

    push(&mut found, &mut fields, &mut values);

    found
}

/// Reads one segment off the values it was written as, and clears them.
fn push(found: &mut Vec<Segment>, fields: &mut [i64; 4], values: &mut Vec<i64>) {
    if values.is_empty() {
        return;
    }

    for (field, delta) in values.iter().enumerate() {
        fields[field] += delta;
    }

    found.push(Segment {
        address: fields[0],
        file: fields[1],
        line: fields[2],
        column: fields[3],
    });
    values.clear();
}
