//! The debug information of a module ([ADR-0025][adr-0025]).
//!
//! A module carries one format and not two. The map of a browser is written by
//! [`crate::sourcemap`]: it points at the sources of a person, and it carries their text
//! itself. The tables of DWARF are written here: a line program that maps code offsets to the
//! source a person reads, a compile unit per module, and a subprogram per function. The
//! tables are written in one pass of the assembler, after the code section is laid out, so that
//! every address is known.
//!
//! A code address of WASM DWARF is an offset from the first byte of the contents of the code
//! section --- the count, which follows the section's `id` and the size ([DWARF for
//! WebAssembly][spec], the convention LLVM emits and `wasm-tools addr2line` reads). A function
//! begins at the first byte of its body content, the locals declaration that its length prefix
//! introduces; a row of the line program is that plus the offset of the instruction inside the
//! body. The code section of a module is a byte count of the bodies the assembler writes, so the
//! layout is computed here in the same order the assembler writes it.
//!
//! [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
//! [spec]: https://yurydelendik.github.io/webassembly-dwarf/

use std::{collections::BTreeMap, sync::Arc};

use gimli::{
    Encoding, Format, LineEncoding, LittleEndian,
    constants::{
        DW_AT_comp_dir, DW_AT_external, DW_AT_high_pc, DW_AT_low_pc, DW_AT_name, DW_AT_producer,
        DW_TAG_subprogram,
    },
    write::{Address, AttributeValue, DwarfUnit, EndianVec, LineProgram, LineString, Sections},
};
use mlkc_line_index::LineIndex;
use mlkc_span::FileId;

use crate::module::{EmittedFunction, ModuleMir};

/// What the debug information reads of the files a body was read from
/// ([ADR-0025][adr-0025]).
///
/// The driver hands this to codegen as part of the input of the stage ([ADR-0009][adr-0009]);
/// codegen never reads a file, and a line of a body is a fact of the input like any other.
///
/// [adr-0009]: ../../docs/adr/0009-pass-contract.md
/// [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
#[derive(Debug, Clone, Default)]
pub struct Sources {
    /// The path of the file the module is a source of: the primary source of the line program.
    primary: String,
    /// The path, the text, and the lines of every file a body may name.
    files: BTreeMap<FileId, SourceFile>,
}

/// One file a body was read from: what a debugger shows, and where its lines are.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// The path of the file, as a reader of the debugger sees it.
    pub path: String,
    /// The text of the file, which the source map carries to a browser ([ADR-0025][adr-0025]).
    ///
    /// [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
    pub text: Arc<str>,
    /// The line and the column of every offset of the file.
    pub lines: Arc<LineIndex>,
}

impl Sources {
    /// The sources of a module whose primary file is `primary`.
    pub fn new(primary: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            files: BTreeMap::new(),
        }
    }

    /// Records the path, the text, and the lines of one file.
    ///
    /// A file that is recorded twice is the same file: the driver holds one text per file, and
    /// a body that names it twice names it once ([ADR-0007][adr-0007]).
    ///
    /// [adr-0007]: ../../docs/adr/0007-vfs-file-state.md
    pub fn insert(
        &mut self,
        file: FileId,
        path: impl Into<String>,
        text: Arc<str>,
        lines: Arc<LineIndex>,
    ) {
        self.files.insert(file, SourceFile {
            path: path.into(),
            text,
            lines,
        });
    }

    /// What is known of one file, if it is known at all.
    pub(crate) fn get(&self, file: FileId) -> Option<&SourceFile> {
        self.files.get(&file)
    }
}

/// The DWARF custom sections of a module, in the order the assembler adds them.
///
/// Empty when there is nothing to describe: a module without a function, or one whose bodies
/// carry no origin at all, has no line to point at.
pub(crate) fn sections(
    module: &ModuleMir,
    functions: &[EmittedFunction],
    sources: &Sources,
    layout: &Layout,
) -> Vec<(&'static str, Vec<u8>)> {
    if functions.is_empty()
        || !functions
            .iter()
            .any(|function| !function.artifact.origins.is_empty())
    {
        return Vec::new();
    }

    let mut unit = DwarfUnit::new(encoding());

    unit.unit.line_program = line_program(functions, sources, layout);

    let root = unit.unit.root();

    unit.unit
        .get_mut(root)
        .set(DW_AT_name, string(&module.name));
    unit.unit.get_mut(root).set(DW_AT_comp_dir, string("/"));
    unit.unit.get_mut(root).set(DW_AT_producer, string("mlkc"));

    for (at, function) in functions.iter().enumerate() {
        let at = layout.functions[at];
        let subprogram = unit.unit.add(root, DW_TAG_subprogram);

        unit.unit
            .get_mut(subprogram)
            .set(DW_AT_name, string(&function.name));
        unit.unit.get_mut(subprogram).set(
            DW_AT_low_pc,
            AttributeValue::Address(Address::Constant(u64::from(at.content))),
        );
        unit.unit
            .get_mut(subprogram)
            .set(DW_AT_high_pc, AttributeValue::Udata(u64::from(at.body)));

        if function.exported {
            unit.unit
                .get_mut(subprogram)
                .set(DW_AT_external, AttributeValue::Flag(true));
        }
    }

    let mut sections = Sections::new(EndianVec::new(LittleEndian));

    unit.write(&mut sections)
        .expect("writing DWARF into a buffer to never fail");

    let mut written = Vec::new();

    sections
        .for_each(|id, data| {
            if !data.slice().is_empty() {
                written.push((id.name(), data.slice().to_vec()));
            }

            Ok::<(), ()>(())
        })
        .expect("collecting the written sections to never fail");

    written
}

/// Where every function stands in the code section, in the offsets DWARF counts from: the
/// contents of the section, which begin at the count of the bodies.
pub(crate) struct Layout {
    /// One entry per function, in the order the assembler writes them.
    pub(crate) functions: Vec<At>,
}

/// One function in the code section.
#[derive(Debug, Clone, Copy)]
pub(crate) struct At {
    /// The offset of the body content: where the function begins.
    pub(crate) content: u32,
    /// The length of the body, without its prefix: how far the function reaches.
    pub(crate) body: u32,
}

impl Layout {
    /// Reads the layout off the bodies: the count, then a prefix and a body per function.
    ///
    /// The offsets are from the first byte of the contents of the code section, the count of the
    /// bodies; each function begins at its own body content, after its length prefix.
    pub(crate) fn of(functions: &[EmittedFunction]) -> Self {
        let count = functions.len() as u32;
        let mut at = leb_len(count);
        let mut functions_at = Vec::with_capacity(functions.len());

        for function in functions {
            let body = function.artifact.body.len() as u32;
            let prefix = leb_len(body);

            functions_at.push(At {
                content: at + prefix,
                body,
            });

            at += prefix + body;
        }

        Self {
            functions: functions_at,
        }
    }

    /// The length of the contents of the code section: the count and the bodies.
    pub(crate) fn payload(&self) -> u32 {
        leb_len(self.functions.len() as u32)
            + self
                .functions
                .iter()
                .map(|at| leb_len(at.body) + at.body)
                .sum::<u32>()
    }
}

/// The line program of a module: one sequence per function, one row per origin.
fn line_program(functions: &[EmittedFunction], sources: &Sources, layout: &Layout) -> LineProgram {
    let mut program = LineProgram::new(
        encoding(),
        LineEncoding::default(),
        LineString::String(b"/".to_vec()),
        None,
        LineString::String(sources.primary.clone().into_bytes()),
        None,
    );

    // Every file an origin names is a file entry; a row names one by its place in the program.
    let mut files: BTreeMap<FileId, gimli::write::FileId> = BTreeMap::new();
    let directory = program.default_directory();

    for artifact in functions {
        for origin in &artifact.artifact.origins {
            if origin.span.is_dummy() || files.contains_key(&origin.span.file) {
                continue;
            }

            let source = sources
                .get(origin.span.file)
                .expect("the driver hands the lines of every file a body names");
            let id = program.add_file(
                LineString::String(source.path.clone().into_bytes()),
                directory,
                None,
            );

            files.insert(origin.span.file, id);
        }
    }

    for (index, artifact) in functions.iter().enumerate() {
        let at = layout.functions[index];

        program.begin_sequence(Some(Address::Constant(u64::from(at.content))));

        // The prologue of a body ends where the body begins: a function of the language has no
        // setup of its own. A debugger that is not told so --- `lldb` --- takes the leading rows
        // of the line the body begins with for a prologue, and slides a breakpoint on that line
        // past them; the end of the prologue is marked on every leading row of that line,
        // because which of them a debugger reads first is not the compiler's to say: a native
        // instruction is not generated for every row.
        let mut first_line = 0_u64;
        let mut leading = true;

        for origin in &artifact.artifact.origins {
            if origin.span.is_dummy() {
                continue;
            }

            let source = sources
                .get(origin.span.file)
                .expect("the driver hands the lines of every file a body names");
            let position = source.lines.line_col(origin.span.start());
            // DWARF counts lines and columns from one; the index counts them from zero.
            let line = u64::from(position.line) + 1;

            if leading {
                if first_line == 0 {
                    first_line = line;
                } else if line != first_line {
                    leading = false;
                }
            }

            let row = program.row();

            // A row is an offset from the body, which begins where the sequence does.
            row.address_offset = u64::from(origin.offset);
            row.file = files[&origin.span.file];
            row.line = line;
            row.column = u64::from(position.col) + 1;
            row.prologue_end = leading;

            program.generate_row();
        }

        program.end_sequence(u64::from(at.body));
    }

    program
}

/// The encoding of the tables: 32-bit DWARF, version 5, addresses of a wasm32 code section.
fn encoding() -> Encoding {
    Encoding {
        address_size: 4,
        format: Format::Dwarf32,
        version: 5,
    }
}

/// A name as an inline DWARF string.
fn string(name: &str) -> AttributeValue {
    AttributeValue::String(name.as_bytes().to_vec())
}

/// The number of bytes a `u32` takes as an unsigned LEB128, which is how WASM writes lengths.
pub(crate) fn leb_len(mut value: u32) -> u32 {
    let mut len = 1;

    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }

    len
}
