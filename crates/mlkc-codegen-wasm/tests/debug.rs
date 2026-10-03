//! The debug tables of a compiled module ([ADR-0023][adr-0023]).
//!
//! A module assembled at the `Lines` level or above carries DWARF custom sections, and this is
//! what reads them back: the line program points at the file and the line a body was read from,
//! and every function is a subprogram over its body. What the tables cannot say alone is where a
//! body stands in the code section, so the addresses are checked against the bytes the assembler
//! wrote --- the same comparison `wasm-tools addr2line` makes, in the convention LLVM emits:
//! a code address is an offset from the contents of the code section, and a function begins at
//! its body content, after the length prefix.
//!
//! [adr-0023]: ../../../docs/adr/0023-debug-information.md

mod harness;

use std::{collections::BTreeMap, ops::Range};

use gimli::{
    AttributeValue, DebuggingInformationEntry, Dwarf, EndianSlice, LittleEndian,
    constants::{DW_AT_high_pc, DW_AT_low_pc, DW_AT_name, DW_TAG_subprogram},
};
use mlkc_codegen_wasm::DebugLevel;
use wasmparser::{Parser, Payload};

/// What the tables are read with: the bytes of the module, little endian as WASM writes them.
type Slice<'a> = EndianSlice<'a, LittleEndian>;

/// A program whose bodies stand on known lines: each function's literal is on a line of its own.
const SOURCE: &str =
    "//- /main.mlk\npub fun first(): Int =\n    1\n\npub fun second(): Int =\n    2\n";

/// The debug tables of a module follow the level it is assembled at.
#[test]
fn the_debug_tables_follow_the_level() {
    let none = harness::project_with(SOURCE, DebugLevel::None);
    let lines = harness::project_with(SOURCE, DebugLevel::Lines);
    let full = harness::project_with(SOURCE, DebugLevel::Full);

    let sections = custom_sections(&none.wasm.bytes);

    assert!(
        sections.keys().all(|name| !name.starts_with(".debug")),
        "a module without debug information to carry no debug tables: {:?}",
        sections.keys().collect::<Vec<_>>(),
    );
    assert!(
        sections.contains_key("name"),
        "the name section to be a part of the module at every level",
    );

    none.validate();

    for compiled in [&lines, &full] {
        let sections = custom_sections(&compiled.wasm.bytes);

        for name in [".debug_abbrev", ".debug_info", ".debug_line"] {
            assert!(
                sections.contains_key(name),
                "`{name}` to be written at this level: {:?}",
                sections.keys().collect::<Vec<_>>(),
            );
        }

        compiled.validate();
    }
}

/// Every function is a subprogram that covers the body it was emitted as.
#[test]
fn every_function_is_a_subprogram_over_its_body() {
    let compiled = harness::project_with(SOURCE, DebugLevel::Lines);
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
    let compiled = harness::project_with(SOURCE, DebugLevel::Lines);
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
