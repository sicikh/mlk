//! The driver, for a browser.
//!
//! This is the wasm host of [`mlkc_driver`]: the same driver the CLI drives, with the text of a
//! buffer pushed into it and the values of the pipeline pulled out. Nothing here knows about a
//! file system --- in a browser there is none, and the driver never wanted one.
//!
//! The boundary is the driver's own: one method per value a host may ask for --- the concrete
//! tree, the typed view, the HIR, the types of a module, the MIR in both of its forms, the LIR
//! the backend lowers it into, the WASM a module assembles to, the program a host runs or
//! builds, and the diagnostics --- and a pull computes what it answers and nothing else. A
//! host that shows the trees only when a person opens them asks for them only then, and the
//! pipeline under the boundary recomputes nothing it already holds. No method bundles the
//! others: what a host does not ask for is not built, and a host that speaks another protocol
//! --- an editor, over LSP --- asks for the same pulls.
//!
//! The standard library is the one thing a host does not push: it is part of the compiler, and
//! a browser has nowhere to read it from, so a host asks the driver for it
//! ([`WasmDriver::use_std`]) and is handed the files it is made of. The library is the
//! compiler's, so the editor shows it as a buffer and writes in none of it.
//!
//! The trees cross the boundary in the shape the syntax tree itself defines:
//! a node is its kind, its range and its children, a token is its kind, its range and its text.
//! A host walks that as deeply as it likes --- folds it, prints it, jumps from it to the text ---
//! and no conversion here decides what a tree is.

use std::sync::Arc;

use mlkc_codegen_wasm::{Sources, compile_module};
use mlkc_driver::{
    DebugInfo, Driver, LinkPlan, Lowered, Manifest, OptLevel, Options, Parse, codegen_diagnostic,
};
use mlkc_hir_def::{ItemLoc, ItemLocLike, ModuleId, Name, ProjectData, ProjectId, dump};
use mlkc_hir_ty::Ty;
use mlkc_line_index::LineIndex;
use mlkc_mir::{
    Bodies, CodeRef, FunctionLoc as MirFunctionLoc, LambdaId, Rvalue, Stmt, StmtKind, Terminator,
    ValueId, cfg::Cfg, dump as mir_dump,
};
use mlkc_span::Span;
use mlkc_syntax::{ModuleRoot, SyntaxNode, TextRange};
use mlkc_vfs::{FileId, VfsPath};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_encoder::{
    AbstractHeapType, CodeSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, Instruction, Module as ModuleEncoding, RefType, TypeSection,
    ValType,
};

/// The clock of the page, which is the clock a driver in a browser is measured by.
///
/// A browser has no time of its own inside wasm, and this is the time it does have: the
/// milliseconds `performance.now()` counts, which is what a look at the driver costs in the
/// end. A host that is not a browser --- the tests of this crate --- leaves the driver without
/// a clock, and reads the work of the passes rather than the time they took.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
}

/// The name of the project the editor's buffers are modules of.
///
/// A browser has one project rather than a manifest: every buffer an editor pushes is a module
/// of this one, and the library is the other project of the world.
const PROJECT: &str = "app";

/// A driver a browser can own.
#[wasm_bindgen]
pub struct WasmDriver {
    driver: Driver,
}

/// A driver the page times: a browser has no clock inside wasm, so it hands the driver the one
/// it has (`performance.now`).
#[cfg(target_arch = "wasm32")]
fn timed_driver() -> Driver {
    let mut driver = Driver::new();

    driver.set_clock(now);

    driver
}

/// A driver that is not timed: the module was not built for a browser, which is what the tests
/// of this crate are, and nothing there measures a pass.
#[cfg(not(target_arch = "wasm32"))]
fn timed_driver() -> Driver {
    Driver::new()
}

#[wasm_bindgen]
impl WasmDriver {
    /// A driver that knows nothing: a host pushes the buffers it holds, and asks for the
    /// standard library of the language when it wants it ([`WasmDriver::use_std`]).
    ///
    /// The driver measures the passes it runs by the clock of the page
    /// ([`mlkc_driver::Driver::set_clock`]).
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut driver = timed_driver();

        // The buffers an editor pushes are modules of the project of the page, and the project
        // depends on the library: a path of it rooted at `std` resolves through the dependency
        // the way a project of a manifest resolves one ([ADR-0016]).
        //
        // [adr-0016]: ../../docs/adr/0016-inter-module-resolution.md
        let mut data = ProjectData::default();
        data.dependencies.insert(
            Name::new(mlkc_stdlib::PROJECT),
            ProjectId::new(mlkc_stdlib::PROJECT),
        );
        driver.set_project(ProjectId::new(PROJECT), data);

        Self { driver }
    }

    /// Records the standard library of the language in the driver, and hands over the files it
    /// is made of: the path each is known by, and its text.
    ///
    /// The library is part of the compiler, and a browser has nowhere to read it from, so a
    /// host asks the driver for it rather than pushing text of its own. What comes back is what
    /// a host shows: the library is the compiler's, and nothing in it is a person's to write.
    ///
    /// The name a host sees is `useStd`: wasm-bindgen keeps the Rust name otherwise,
    /// and the editor around this module is written in JavaScript.
    #[wasm_bindgen(js_name = useStd)]
    pub fn use_std(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.register_library())
    }

    /// The canonical name of the project the editor writes in: what a host names the files of a
    /// build by, and the archive of the sources beside them ([ADR-0021]).
    ///
    /// A browser holds one project, which is why nothing a host pushes names one: this is that
    /// name, and the editor names a build and the sources of it after it.
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    pub fn project(&self) -> String {
        PROJECT.to_owned()
    }

    /// Configures the pipeline: what debug information a module carries, and how hard the
    /// passes optimize ([ADR-0025]).
    ///
    /// The formats cross the boundary as the words a host writes: `none`, `source-map`,
    /// `dwarf-lines`, and `dwarf-full` for the debug information of a module; `none` and
    /// `full` for the optimization. Returns whether the options changed, which is what tells
    /// a host to read what depends on them again.
    ///
    /// [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
    #[wasm_bindgen(js_name = setOptions)]
    pub fn set_options(&mut self, debug: &str, opt: &str) -> Result<bool, JsValue> {
        let debug = match debug {
            "none" => DebugInfo::None,
            "source-map" => DebugInfo::SourceMap,
            "dwarf-lines" => DebugInfo::DwarfLines,
            "dwarf-full" => DebugInfo::DwarfFull,
            other => return Err(failure(&format!("no debug option is called `{other}`"))),
        };
        let opt = match opt {
            "none" => OptLevel::None,
            "full" => OptLevel::Full,
            other => {
                return Err(failure(&format!(
                    "no optimization level is called `{other}`"
                )));
            },
        };

        Ok(self.driver.set_options(Options { debug, opt }))
    }

    /// Feeds the text of a file into the driver; `null` or `undefined` means the file is gone.
    ///
    /// Returns whether the contents changed.
    /// Pushing the same text again changes nothing and rebuilds nothing,
    /// which is what makes "analyze the buffer after every keystroke" affordable.
    ///
    /// The name a host sees is `setText`: wasm-bindgen keeps the Rust name otherwise,
    /// and the editor around this module is written in JavaScript.
    #[wasm_bindgen(js_name = setText)]
    pub fn set_text(&mut self, path: &str, text: Option<String>) -> bool {
        let path = path_of(path);
        let there = text.is_some();
        let changed = self.driver.set_file_text(path.clone(), text);

        // What a host pushes is a module of the project of the page: the driver holds no
        // manifest, and a buffer with no project would have no name another module could call
        // it by. What the graph already says is read rather than overwritten: a file the compiler
        // reads as one of another project --- a file of the library --- stays there, and a file
        // that is gone is not one of the page's project any more, because a module is a file
        // ([ADR-0021]).
        //
        // The id is read whether or not the file is there: a file that is gone keeps its id,
        // and it is the only name the module it was has left ([`Driver::path_id`]).
        //
        // [adr-0021]: ../../docs/adr/0021-translation-units.md
        let page = ProjectId::new(PROJECT);

        if let Some(file) = self.driver.path_id(&path) {
            let module = ModuleId(file);
            let held = self.driver.project_graph().project_of(module).cloned();

            match (there, held) {
                // A file that is gone is not one of the page's project any more.
                (false, Some(project)) if project == page => {
                    self.driver.remove_module_project(module);
                },
                // A file no project claims is one of the page's; a file of another project --- a
                // file of the library --- stays where the compiler put it.
                (true, None) => {
                    self.driver.set_module_project(module, page);
                },
                _ => {},
            }
        }

        changed
    }

    /// The concrete syntax tree of a buffer: lossless, tokens and trivia included.
    ///
    /// The tree is a value a host navigates, not text it re-parses:
    /// a node is its kind, its range and its children, a token is its kind, its range and its text.
    ///
    /// Throws when the driver holds no text for the file:
    /// an editor pushes the buffer before it asks about it.
    pub fn cst(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.cst_of(path)?)
    }

    /// The typed view over the same tree, or `null` when the parse found no module root.
    ///
    /// Its shape is the one the typed tree gives itself: a field of the compiler's AST is a key
    /// here, a list is an array, and a token is a node of the concrete tree.
    pub fn ast(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.ast_of(path)?)
    }

    /// The HIR of the module in the buffer, or `null` when there is nothing to lower.
    ///
    /// The lines are read the way a person reads the HIR --- the module and its items, and then
    /// a body per entity that owns one --- and each line says where the node it is about is
    /// written, which is what an editor marks the buffer by.
    pub fn hir(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.hir_of(path)?)
    }

    /// What checking the types of the buffer left, or `null` when there is nothing to check.
    ///
    /// The types are read where a person reads them: the type an entity of the module was
    /// resolved to, and the type every node of every body was checked to.
    pub fn types(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.types_of(path)?)
    }

    /// The MIR of the module in the buffer, in the CFG form, or `null` when there is nothing to
    /// lower ([ADR-0019](../../docs/adr/0019-mir.md)).
    ///
    /// The bodies are read the way a person reads them --- a block with its parameters, the
    /// statements of it, and the terminator it ends in --- and every line says where it was read
    /// from, which is what an editor marks the buffer by. A body whose check reported a mistake
    /// has no MIR, and is not among the bodies.
    pub fn mir(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.mir_of(path, Form::Cfg)?)
    }

    /// The MIR of the module in the buffer, in the SSA form: the CFG form with block parameters
    /// ([ADR-0019](../../docs/adr/0019-mir.md)).
    ///
    /// The name a host sees is `mirSsa`: wasm-bindgen keeps the Rust name otherwise.
    #[wasm_bindgen(js_name = mirSsa)]
    pub fn mir_ssa(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.mir_of(path, Form::Ssa)?)
    }

    /// The LIR of the module in the buffer: the target's instructions in SSA form, ready to
    /// encode ([ADR-0022](../../docs/adr/0022-wasm-lir.md)).
    ///
    /// The bodies are read the way a person reads them --- a block with its parameters, the
    /// instructions of it, and the terminator it ends in --- and every line says where it was
    /// read from, which is what an editor marks the buffer by. The table of a body says where
    /// every value that needs storage lives, and which values are emitted where they are read.
    pub fn lir(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.lir_of(path)?)
    }

    /// The WASM the module in the buffer assembles to, as text ([ADR-0020]).
    ///
    /// The same bytes the link stage hands a host, printed as the WebAssembly text format:
    /// the types, the imports, the exports, and the body of every function, which is what a
    /// person reads a module by. `null` when there is nothing to compile.
    ///
    /// [adr-0020]: ../../docs/adr/0020-wasm-backend.md
    pub fn wat(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.wat_of(path)?)
    }

    /// The program the buffers make, as the manifest of a run ([ADR-0021]).
    ///
    /// A run is of the whole project rather than of one buffer: every module of it is compiled,
    /// the modules of the library it depends on included, and what comes back is what a host
    /// needs to run it --- the modules in the order they are instantiated in, what each of them
    /// imports and exports, the module of the host functions every extern is carried over the
    /// boundary by, and the `#[entry]` a host calls to begin.
    ///
    /// Nothing is instantiated here: a module of the language is instantiated by the host that
    /// runs the program, which wires the exports of a module to the imports of the next
    /// ([ADR-0021]).
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    #[wasm_bindgen(js_name = run)]
    pub fn run(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.program_of()?)
    }

    /// The program the buffers make, built and linked, without running it ([ADR-0021]).
    ///
    /// A build is the manifest of a run and nothing done with it: nothing is instantiated, and
    /// the entry point is not called. A host that keeps the program rather than runs it asks
    /// for this --- the editor, which packs every module of it into an archive.
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    #[wasm_bindgen(js_name = build)]
    pub fn build(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.program_of()?)
    }

    /// What the stages of the pipeline reported, in the order they reported it.
    ///
    /// The diagnostics are what an editor marks the buffer with: the level, the kind, and the
    /// places they point at, each of them with the line and the column it sits at.
    pub fn diagnostics(&mut self, path: &str) -> Result<JsValue, JsValue> {
        to_js(&self.diagnostics_of(path)?)
    }

    /// What the driver did since this was last asked, by pass and by unit: what it handed over
    /// from a slot, what it read again, what it kept although it read again, what it dropped,
    /// and how long the passes it ran took.
    ///
    /// Reading the counters is what clears them, so a host that wants to know what one pull
    /// cost asks for them right after it: what comes back is that pull's work, and the time it
    /// took is the host's own clock around the call. A unit is what a pass is a value of: the
    /// path of a file or a module, the name of a project, and the name of a body and where it
    /// is written.
    ///
    /// The name a host sees is `stats`: wasm-bindgen keeps the Rust name otherwise.
    #[wasm_bindgen(js_name = stats)]
    pub fn stats(&mut self) -> Result<JsValue, JsValue> {
        to_js(&Stats::of(&mut self.driver))
    }
}

impl WasmDriver {
    /// Records the standard library, and hands over the files it is made of.
    fn register_library(&mut self) -> Vec<StdFile> {
        self.driver
            .use_std()
            .into_iter()
            .map(|file| {
                StdFile {
                    path: file.path.to_string(),
                    text: file.text,
                }
            })
            .collect()
    }

    /// The concrete syntax tree of a buffer.
    fn cst_of(&mut self, path: &str) -> Result<SyntaxNode, JsValue> {
        Ok(self.parse_of(path)?.syntax())
    }

    /// The typed view over the same tree.
    fn ast_of(&mut self, path: &str) -> Result<Option<ModuleRoot>, JsValue> {
        Ok(self.parse_of(path)?.module_root())
    }

    /// The HIR of the module in the buffer.
    fn hir_of(&mut self, path: &str) -> Result<Option<Hir>, JsValue> {
        let file = self.file(path)?;

        Ok(self.driver.lower(file).map(|lowered| Hir::of(&lowered)))
    }

    /// What checking the types of the buffer left.
    fn types_of(&mut self, path: &str) -> Result<Option<Types>, JsValue> {
        let file = self.file(path)?;

        let Some(lowered) = self.driver.lower(file) else {
            return Ok(None);
        };

        Ok(Types::of(&mut self.driver, ModuleId(file), &lowered))
    }

    /// The MIR of every body of the module in the buffer, in the form asked for.
    fn mir_of(&mut self, path: &str, form: Form) -> Result<Option<Mir>, JsValue> {
        let file = self.file(path)?;

        // A file the driver does not lower has no MIR to read: the parse reported a mistake, or
        // found no module. A body of a file that lowers but does not check clean is left out by
        // the driver ([ADR-0019](../../docs/adr/0019-mir.md)).
        let Some(lowered) = self.driver.lower(file) else {
            return Ok(None);
        };

        Ok(Some(Mir::of(&mut self.driver, &lowered, form)))
    }

    /// The LIR of every body of the module in the buffer.
    fn lir_of(&mut self, path: &str) -> Result<Option<Lir>, JsValue> {
        let file = self.file(path)?;

        // A file the driver does not lower has nothing to lower into the LIR, and a body whose
        // module is not whole is left out ([ADR-0022](../../docs/adr/0022-wasm-lir.md)).
        let Some(lowered) = self.driver.lower(file) else {
            return Ok(None);
        };

        Ok(Some(Lir::of(&mut self.driver, &lowered)))
    }

    /// The WASM of the module in the buffer, as text.
    fn wat_of(&mut self, path: &str) -> Result<Option<Wat>, JsValue> {
        let file = self.file(path)?;

        // A module that is not whole has nothing to compile: the parse found no module, or a
        // body of it is one the checker did not read clean ([ADR-0020]).
        //
        // [adr-0020]: ../../docs/adr/0020-wasm-backend.md
        let Some(module) = self.driver.mir_module(ModuleId(file)) else {
            return Ok(None);
        };

        // A body the driver did not lower has no LIR, and a module that is not whole has
        // nothing to compile ([ADR-0022]).
        //
        // [adr-0022]: ../../docs/adr/0022-wasm-lir.md
        let Some(lirs) = self.driver.module_lir(ModuleId(file)) else {
            return Ok(None);
        };

        let options = self.driver.options();
        let sources = self.sources_of(&module);
        let (wasm, reports) = compile_module(&module, &lirs, options.debug, &sources);
        let index = self
            .driver
            .line_index(file)
            .ok_or_else(|| failure(&format!("{path} has no lines to read")))?;
        let text = wat_text(&wasm.bytes)
            .map_err(|error| failure(&format!("the module of {path} did not print: {error}")))?;
        let sections = custom_sections(&wasm.bytes);
        let diagnostics = reports
            .iter()
            .map(|report| Diagnostic::of(&codegen_diagnostic(report), &index))
            .collect();

        Ok(Some(Wat {
            text,
            sections,
            diagnostics,
        }))
    }

    /// What the debug information of a module reads: the path, the text, and the lines of the
    /// file it was read from ([ADR-0025][adr-0025]).
    ///
    /// Every function of the module names the same file today, and a body may name another
    /// one once inlining moves code across modules ([ADR-0022][adr-0022]); the page hands the
    /// driver the text of every buffer, so the lines of any of them are a pull away, and so is
    /// the text a source map carries.
    ///
    /// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
    /// [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
    fn sources_of(&mut self, module: &mlkc_codegen_wasm::ModuleMir) -> Sources {
        let Some(first) = module.functions.first() else {
            return Sources::default();
        };
        let file = first.function.module().0;
        let path = self.driver.file_path(file).to_string();
        let mut sources = Sources::new(path.clone());

        let mut seen = std::collections::BTreeSet::new();

        for function in &module.functions {
            let file = function.function.module().0;

            if !seen.insert(file) {
                continue;
            }

            let Some(lines) = self.driver.line_index(file) else {
                continue;
            };
            let Some(text) = self.driver.file_text(file) else {
                continue;
            };

            sources.insert(file, self.driver.file_path(file).to_string(), text, lines);
        }

        sources
    }

    /// The program of the project of the page: every module of the project and of the projects
    /// it depends on, compiled and linked, as the manifest a run is of.
    ///
    /// The manifest is one value whichever way a host takes it: `run` instantiates it and calls
    /// the entry point, and `build` keeps it.
    fn program_of(&mut self) -> Result<Run, JsValue> {
        let plan = self.driver.link(&ProjectId::new(PROJECT)).ok_or_else(|| {
            failure("the program does not link: fix what the compiler reported and run again")
        })?;
        let diagnostics = plan
            .diagnostics
            .iter()
            .map(|diagnostic| self.render(diagnostic))
            .collect();

        Ok(run_manifest(PROJECT, &plan, diagnostics))
    }

    /// A diagnostic of the pipeline as a host reads it.
    ///
    /// A diagnostic of a run may point at more than one buffer — the two entries of a project
    /// are two modules — so the lines of a label are read from the file it is in.
    fn render(&mut self, diagnostic: &mlkc_diagnostics::Diagnostic) -> Diagnostic {
        Diagnostic::of_files(diagnostic, |file| self.driver.line_index(file))
    }

    /// What the stages of the pipeline reported.
    fn diagnostics_of(&mut self, path: &str) -> Result<Vec<Diagnostic>, JsValue> {
        let file = self.file(path)?;
        let diagnostics = self
            .driver
            .diagnostics(file)
            .ok_or_else(|| failure(&format!("{path} has no diagnostics to read")))?;
        let index = self
            .driver
            .line_index(file)
            .ok_or_else(|| failure(&format!("{path} has no lines to read")))?;

        Ok(diagnostics
            .iter()
            .map(|it| Diagnostic::of(it, &index))
            .collect())
    }

    /// The parse of a buffer, or a thrown error when the driver holds no text for it.
    fn parse_of(&mut self, path: &str) -> Result<Arc<Parse>, JsValue> {
        let file = self.file(path)?;

        self.driver
            .parse(file)
            .ok_or_else(|| failure(&format!("{path} is not text the driver can parse")))
    }

    /// The id the driver knows a path under, or a thrown error when it holds nothing for it.
    fn file(&self, path: &str) -> Result<FileId, JsValue> {
        self.driver
            .file_id(&path_of(path))
            .ok_or_else(|| failure(&format!("no file was pushed at {path}")))
    }
}

impl Default for WasmDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// The WASM of one module, as a host reads it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Wat {
    /// The module in the WebAssembly text format.
    ///
    /// The `(@custom ...)` groups of the debug sections are left out: they are binary tables
    /// with nothing a person reads, and [`Wat::sections`] says what they are and how large.
    text: String,

    /// The custom sections of the module, in the order it carries them.
    sections: Vec<WatSection>,

    /// What the back end reported about the bodies of the module.
    diagnostics: Vec<Diagnostic>,
}

/// One custom section of a module: its name and its size in bytes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WatSection {
    /// The name of the section: `name`, `.debug_info`, ...
    name: String,

    /// How many bytes of the module the section is.
    size: u32,
}

/// The module in the WebAssembly text format, without the binary custom sections.
fn wat_text(bytes: &[u8]) -> Result<String, String> {
    let text = wasmprinter::print_bytes(bytes).map_err(|error| error.to_string())?;
    let mut kept = String::with_capacity(text.len());

    for line in text.lines() {
        if is_debug_custom(line) {
            continue;
        }

        kept.push_str(line);
        kept.push('\n');
    }

    Ok(kept)
}

/// Whether the name of a custom section is a table of DWARF.
fn is_debug_section(name: &str) -> bool {
    name.starts_with(".debug_") || name == "external_debug_info"
}

/// Whether a line of WAT is a debug custom section with its binary contents.
fn is_debug_custom(line: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix("(@custom \"") else {
        return false;
    };
    let name = rest.split('"').next().unwrap_or("");

    is_debug_section(name)
}

/// The custom sections of a module, read off its bytes.
fn custom_sections(bytes: &[u8]) -> Vec<WatSection> {
    let mut sections = Vec::new();

    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        let Ok(wasmparser::Payload::CustomSection(reader)) = payload else {
            continue;
        };

        sections.push(WatSection {
            name: reader.name().to_owned(),
            size: reader.data().len() as u32,
        });
    }

    sections
}

/// The manifest of a run: what a host instantiates, and where the program begins ([ADR-0021]).
///
/// A run is not a merged binary but a list: a host walks the modules in the order they are
/// given, which is a provider before the modules that import it, hands the exports of a module
/// to the imports of the next one, and calls the entry point. The host functions of the program
/// are one module the host instantiates first, and their bytes are here with the rest: what
/// crosses the boundary as an `i32` is a word taken apart ([`host_module`]).
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Run {
    /// What a host that writes the program out as files reads: the project, the file every
    /// module is written as, and where the program begins ([`Manifest`]).
    manifest: Manifest,

    /// The modules of the program, providers before the modules that import them.
    modules: Vec<RunModule>,

    /// The module of the host functions the externs of the program are implemented by.
    host: Vec<u8>,

    /// The module and the function a host calls to run the program, when it declares an entry.
    entry: Option<RunEntry>,

    /// What the code generator and the link stage reported about the program.
    diagnostics: Vec<Diagnostic>,

    /// What this host cannot do for the program: an extern none of its functions implements.
    problems: Vec<String>,
}

/// One module of a run: its bytes, and what they import and export.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunModule {
    /// The canonical name of the module: the name the modules after it import it by, and the
    /// name a stack trace shows.
    name: String,

    /// The bytes of the WASM module: what the debug option asked for is in them, and the
    /// browser runs under the source map ([`DebugInfo::SourceMap`]).
    bytes: Vec<u8>,

    /// The functions the module imports, in the order of their indices.
    imports: Vec<RunImport>,

    /// The functions the module exports, in the order it declares them.
    exports: Vec<RunExport>,
}

/// One function a module of a run imports.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunImport {
    /// The canonical name of the module the function belongs to.
    module: String,

    /// The name of the function inside its module.
    name: String,

    /// Whether the function is declared `#[extern]`: a host implements it, not a module.
    external: bool,

    /// How many words the function takes; it gives back one.
    arity: u32,
}

/// One function a module of a run exports.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunExport {
    /// The name of the function inside its module.
    name: String,

    /// How many words the function takes; it gives back one.
    arity: u32,
}

/// Where a run begins: the module the entry is in, and the name it is exported by.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunEntry {
    /// The canonical name of the module the entry is in.
    module: String,

    /// The name the entry is exported by.
    name: String,
}

/// The manifest of a run of `plan`, and what the host is told about it: the project the plan
/// was linked for, and everything [`Run`] holds.
fn run_manifest(project: &str, plan: &LinkPlan, diagnostics: Vec<Diagnostic>) -> Run {
    let mut modules = Vec::with_capacity(plan.order.len());
    let mut problems = Vec::new();

    for id in &plan.order {
        let module = &plan.modules[id];
        let mut imports = Vec::with_capacity(module.imports.len());

        for import in &module.imports {
            // An extern is what the host functions of a run implement, and nothing else is a
            // host function: a program that imports another name is one this host cannot run.
            if import.external && !HOST_FUNCTIONS.contains(&import.name.as_str()) {
                problems.push(format!(
                    "the host does not implement the extern `{}::{}`",
                    import.module, import.name,
                ));
            }

            imports.push(RunImport {
                module: import.module.clone(),
                name: import.name.clone(),
                external: import.external,
                arity: import.arity,
            });
        }

        modules.push(RunModule {
            name: plan.names[id].clone(),
            bytes: module.bytes.clone(),
            imports,
            exports: module
                .exports
                .iter()
                .map(|export| {
                    RunExport {
                        name: export.name.clone(),
                        arity: export.arity,
                    }
                })
                .collect(),
        });
    }

    let entry = plan.entry.as_ref().map(|(module, name)| {
        RunEntry {
            module: plan.names[module].clone(),
            name: name.clone(),
        }
    });

    // An entry the link stage reported a mistake about is a diagnostic a host reads; an entry
    // the project simply does not declare is not a mistake, and it is what a host is told here.
    if entry.is_none() && diagnostics.is_empty() {
        problems.push("the program declares no `#[entry]`".to_owned());
    }

    Run {
        manifest: Manifest::of(project, plan, Some(HOST_FILE)),
        modules,
        host: host_module(),
        entry,
        diagnostics,
        problems,
    }
}

/// The name of the module the host functions of a run are imported under.
///
/// The program never sees it: what a module of the language imports is the extern, and this is
/// the module the shim asks a host for it from.
const HOST_MODULE: &str = "host";

/// The name the module of the host functions is written as, when a build is written as files
/// ([`Manifest::host`]).
const HOST_FILE: &str = "host.wasm";

/// The host functions a run implements, by the name a program imports them under.
///
/// The names are the ones the library declares `#[extern]` ([ADR-0015]); a program that imports
/// any other one is a program this host cannot run, and the manifest says so rather than
/// failing at instantiation.
///
/// [adr-0015]: ../../docs/adr/0015-standard-library.md
const HOST_FUNCTIONS: &[&str] = &["print-int", "print-bool"];

/// The module a host instantiates beside the program: the externs of the language, carried over
/// the boundary as the plain numbers a host outside wasm has ([ADR-0018]).
///
/// A word of the language is an `i31` immediate or a reference, and a JavaScript host can make
/// neither; what crosses this boundary is an `i32`. Every extern of the language takes and
/// gives back an immediate, so the module imports one function of the module `host` per extern,
/// under the name of the extern, and exports the signature the program imports it by: an
/// `(ref i31)` in, an `(ref i31)` out. The export takes the immediate apart, calls the host
/// with the number inside it, and answers the word `Unit` --- the zero immediate --- which is
/// what a function that prints gives back.
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
fn host_module() -> Vec<u8> {
    let immediate = ValType::Ref(RefType::new_abstract(AbstractHeapType::I31, false, false));
    let mut types = TypeSection::new();

    // The type of a function the host implements, and the type of the export carrying it over
    // the boundary: a number in and a number out, an immediate in and an immediate out.
    types.ty().function([ValType::I32], [ValType::I32]);
    types.ty().function([immediate], [immediate]);

    let mut imports = ImportSection::new();

    for name in HOST_FUNCTIONS {
        imports.import(HOST_MODULE, name, EntityType::Function(0));
    }

    // The imports take the indices before the functions the module defines, which is what the
    // function index space of a module is.
    let offset = HOST_FUNCTIONS.len() as u32;
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();

    for (index, name) in HOST_FUNCTIONS.iter().enumerate() {
        functions.function(1);
        exports.export(name, ExportKind::Func, offset + index as u32);
    }

    let mut code = CodeSection::new();

    for index in 0..offset {
        let mut function = Function::new([]);

        function.instruction(&Instruction::LocalGet(0));
        function.instruction(&Instruction::I31GetS);
        function.instruction(&Instruction::Call(index));
        function.instruction(&Instruction::Drop);
        function.instruction(&Instruction::I32Const(0));
        function.instruction(&Instruction::RefI31);
        function.instruction(&Instruction::End);

        // `CodeSection::raw` writes the size prefix of the entry itself.
        code.raw(&function.into_raw_body());
    }

    let mut wasm = ModuleEncoding::new();

    wasm.section(&types);
    wasm.section(&imports);
    wasm.section(&functions);
    wasm.section(&exports);
    wasm.section(&code);

    wasm.finish()
}

/// One file of the standard library, as a host reads it.
#[derive(Serialize)]
struct StdFile {
    /// The path the driver knows the file by, which is the one a host shows.
    path: String,

    /// The source of it.
    text: &'static str,
}

/// The HIR of one module, as a host reads it.
#[derive(Serialize)]
struct Hir {
    /// The lines of the HIR, in the order a reader reads them: the module and its items,
    /// and then a body per entity that owns one.
    nodes: Vec<HirNode>,
}

impl Hir {
    /// Reads the HIR of a lowered module the way a host reads it.
    ///
    /// A line of the reading is about a node of the HIR ([`dump::Target`]), and what a host
    /// marks a buffer by is a range of it: the line of the item tree is read against the
    /// syntax of the module, and the line of a body against the places the lowering of that
    /// body recorded.
    fn of(lowered: &Lowered) -> Self {
        let mut nodes = Vec::new();

        for node in dump::item_tree_nodes(lowered.item_tree()) {
            nodes.push(HirNode::of(&node, &|target| surface_range(lowered, target)));
        }

        for body in lowered.bodies() {
            let reading = dump::body_nodes(body.owner(), &body.body().body);
            let places = &body.body().source_map;

            nodes.push(HirNode::of(&reading, &|target| {
                match target {
                    dump::Target::Expr(expr) => places.expr(*expr),
                    dump::Target::Pat(pat) => places.pat(*pat),
                    // What a body names can be a place outside it: the entity of the module.
                    dump::Target::Item(item) => lowered.item_range(item),
                    // A body writes no type: its annotations are the ones of the signature it is
                    // a body of, which the surface of the module is read for.
                    dump::Target::Type { .. } => None,
                }
            }));
        }

        Self { nodes }
    }
}

/// One line of the HIR, and the lines under it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HirNode {
    /// The text of the line: `fun main  @2.0`, `param: Int -> use Int`.
    text: String,

    /// What the line says, as the parts a host paints: a line is one part unless a part of it
    /// says something the host paints differently from the rest, which is what the type
    /// a signature writes does --- a type written as a path reads the way a path reads.
    parts: Vec<HirPart>,

    /// What the line is, which is what a host paints it by.
    kind: &'static str,

    /// The part of the buffer the line is about, in bytes, or nothing where the line is about
    /// something a module did not write: a section header, a field of a declaration.
    range: Option<[u32; 2]>,

    /// The part of the buffer what the line resolves to is written at, when the line is about
    /// a path that named something: the declaration a name comes from, the binding it stands
    /// for. Nothing for a line that names nothing, and for a name of another module, which is
    /// written in a file this one is not marked against.
    resolves: Option<[u32; 2]>,

    /// The lines under it, which a host folds.
    children: Vec<HirNode>,
}

/// One part of the text of a line.
#[derive(Serialize)]
struct HirPart {
    /// What this part of the line says.
    text: String,

    /// What it is, when it is not what the line is; `null` for the part of a line that says
    /// what the line says.
    kind: Option<&'static str>,
}

impl HirNode {
    /// Reads one line of a reading, and everything under it.
    ///
    /// `range` is where the module says a node of the HIR is written, and it is the one thing
    /// the reading of the HIR does not know: a line says what a node is, and the driver is
    /// what says where it was read from.
    fn of(node: &dump::Node, range: &impl Fn(&dump::Target) -> Option<TextRange>) -> Self {
        Self {
            text: node.text(),
            parts: node
                .parts
                .iter()
                .map(|part| {
                    HirPart {
                        text: part.text.clone(),
                        kind: part.kind.map(kind_of),
                    }
                })
                .collect(),
            kind: kind_of(node.kind),
            range: node.target.as_ref().and_then(range).map(covered),
            resolves: node.resolves.as_ref().and_then(range).map(covered),
            children: node
                .children
                .iter()
                .map(|child| Self::of(child, range))
                .collect(),
        }
    }
}

/// Where what a line of the surface of a module is about is written: an entity of the module,
/// or a type written in the declaration of one.
fn surface_range(lowered: &Lowered, target: &dump::Target) -> Option<TextRange> {
    match target {
        dump::Target::Item(item) => lowered.item_range(item),
        dump::Target::Type { item, place } => lowered.type_range(item, *place),
        _ => None,
    }
}

/// What a line of a reading is, as a host reads it.
fn kind_of(kind: dump::NodeKind) -> &'static str {
    match kind {
        dump::NodeKind::Module => "module",
        dump::NodeKind::Body => "body",
        dump::NodeKind::Section => "section",
        dump::NodeKind::Item => "item",
        dump::NodeKind::Field => "field",
        dump::NodeKind::Expr => "expr",
        dump::NodeKind::Pat => "pat",
        dump::NodeKind::Path => "path",
    }
}

/// The part of a buffer a range covers, in the bytes a host counts.
fn covered(range: TextRange) -> [u32; 2] {
    [u32::from(range.start()), u32::from(range.end())]
}

/// What checking the types of a module left, as a host reads it.
///
/// Types are values of the compiler, and what a host shows of them is what a person reads at
/// a place: the type an entity of the module was resolved to, and the type every node of every
/// body was checked to.
#[derive(Serialize)]
struct Types {
    /// The types of the entities of the module, in the order it declares them.
    surface: Vec<SurfaceType>,

    /// The checked bodies, in the order the module declares them.
    bodies: Vec<BodyTypes>,
}

/// What the driver did since a host last read the counters, as it reads it.
///
/// The counters are the driver's account of its own incrementality: what it handed over from
/// a slot, what it had to read again, what it kept although it read again, what went, and how
/// long the passes it ran took. A row is of one pass and one unit, and a host that wants the
/// totals of a pass --- or of a pull --- adds the rows up.
#[derive(Serialize)]
struct Stats {
    /// The counters of every pass and unit, in the order a module is read in.
    rows: Vec<StatsRow>,
}

impl Stats {
    /// The counters the driver holds, and a clean slate for the next read of them.
    fn of(driver: &mut mlkc_driver::Driver) -> Self {
        let taken = driver.take_stats();

        Self {
            rows: taken
                .iter()
                .map(|(pass, unit, tally)| {
                    StatsRow {
                        pass: pass.name().to_owned(),
                        unit: driver.unit_name(unit),
                        hits: tally.hits,
                        misses: tally.misses,
                        stales: tally.stales,
                        kept: tally.kept,
                        dropped: tally.dropped,
                        took: tally.took.as_secs_f64() * 1000.0,
                    }
                })
                .collect(),
        }
    }
}

/// What one pass did for one unit since the counters were last read.
#[derive(Serialize)]
struct StatsRow {
    /// The pass, by the name it is known by: `parse`, `interface`, `check`.
    pass: String,

    /// The unit the pass was asked for: the path of a file or a module, the name of a project,
    /// or the name of a body and where it is written.
    unit: String,

    /// How often the value was there: the slot was keyed by what it was built from.
    hits: u32,

    /// How often the pass ran and the driver held nothing to hand over.
    misses: u32,

    /// How often the pass ran although a value was held, because what the value was built from
    /// had changed.
    stales: u32,

    /// How often a pass that ran read the same as the value the driver held, which is the value
    /// it kept.
    kept: u32,

    /// How many values went, with the input they were built from.
    dropped: u32,

    /// How long the pass spent running for this unit, in milliseconds: zero for a pass that
    /// never ran, and zero for a host that gave the driver no clock.
    took: f64,
}

/// One entity of the surface of a module and the type it was resolved to.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SurfaceType {
    /// What the entity is: `fun main`, `type Point`.
    name: String,

    /// The type it was resolved to, as it reads: `() -> Int`.
    ty: String,

    /// Where the declaration is written, in bytes, or nothing where it is written nowhere.
    range: Option<[u32; 2]>,
}

/// The types of the nodes of one body.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BodyTypes {
    /// The entity the body belongs to: `fun main`.
    owner: String,

    /// Where the declaration that owns the body is written, in bytes.
    range: Option<[u32; 2]>,

    /// The types of the nodes of the body, in the order they are written in.
    nodes: Vec<TypedNode>,
}

/// One node of a body and the type it was checked to.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TypedNode {
    /// What the node is: `expr` or `pat`.
    kind: &'static str,

    /// What a node that is written nowhere is called by: `expr #3`.
    label: String,

    /// The part of the buffer the node covers, in bytes, or nothing when nothing was written.
    range: Option<[u32; 2]>,

    /// The type, as it reads: `Int`, `(Int) -> Bool`, `{error}`.
    ty: String,

    /// Whether the type is the type of a mistake, which a host paints as one.
    error: bool,
}

impl Types {
    /// Reads the types of a module and of its bodies the way a host reads them.
    ///
    /// The driver is what resolved the surface and checked the bodies; what the reading adds is
    /// where each of them was written, which is what the HIR of the module holds --- the range
    /// of an entity, and the source map of a body.
    fn of(driver: &mut Driver, module: ModuleId, lowered: &Lowered) -> Option<Self> {
        let types = driver.module_types(module)?;
        let mut surface = Vec::new();

        for (entity, ty) in types.iter() {
            surface.push(SurfaceType {
                name: entity_name(&entity.item),
                ty: ty.to_string(),
                range: lowered.item_range(&entity.item).map(covered),
            });
        }

        let mut bodies = Vec::new();

        for body in lowered.bodies() {
            let Some(checked) = driver.check(body.owner()) else {
                continue;
            };

            let places = &body.body().source_map;
            let mut nodes: Vec<(u32, u32, TypedNode)> = Vec::new();

            for (id, ty) in checked.expr_types() {
                nodes.push(node("expr", id.into_raw().into_u32(), places.expr(id), ty));
            }

            for (id, ty) in checked.pat_types() {
                nodes.push(node("pat", id.into_raw().into_u32(), places.pat(id), ty));
            }

            // The nodes are read in the order they are written in: a node nothing was read
            // from has no place, and is read after the ones that have one.
            nodes.sort_by_key(|(at, id, _)| (*at, *id));

            let owner = ItemLoc::from(body.owner().item.clone());

            bodies.push(BodyTypes {
                owner: entity_name(&owner),
                range: lowered.item_range(&owner).map(covered),
                nodes: nodes.into_iter().map(|(_, _, node)| node).collect(),
            });
        }

        Some(Self { surface, bodies })
    }
}

/// One node of a body, as a host reads it.
///
/// `at` is where the node is written, and the two numbers a host sorts it by are its place in
/// the buffer --- or the end of it, for a node nothing was read from --- and the position of
/// the node in the body of the compiler, which tells two nodes in one place apart.
fn node(kind: &'static str, id: u32, range: Option<TextRange>, ty: &Ty) -> (u32, u32, TypedNode) {
    let at = range.map_or(u32::MAX, |range| u32::from(range.start()));

    (at, id, TypedNode {
        kind,
        label: format!("{kind} #{}", id - 1),
        range: range.map(covered),
        ty: ty.to_string(),
        error: ty.is_error(),
    })
}

/// Which of the two forms of MIR a host asks for ([ADR-0019](../../docs/adr/0019-mir.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// The CFG form: an assignment writes a slot, and a join is a slot the predecessors wrote.
    Cfg,
    /// The SSA form: every value is defined once, and a join is a block parameter.
    Ssa,
}

impl Form {
    /// The form, by the name a host reads it by.
    const fn name(self) -> &'static str {
        match self {
            Self::Cfg => "cfg",
            Self::Ssa => "ssa",
        }
    }
}

/// The MIR of the bodies of one module, as a host reads it.
///
/// A body is read the way a person reads it: the blocks in the order they are allocated, the
/// statements of a block in the order they run, and the terminator the block ends in. Every
/// line and every value carries the range of the buffer it was read from, which is what an
/// editor marks while a pointer is on the line.
#[derive(Serialize)]
struct Mir {
    /// Which form the bodies are read in: `cfg` or `ssa`.
    form: &'static str,

    /// The bodies of the module that check clean, in the order it declares them.
    bodies: Vec<MirBody>,
}

impl Mir {
    /// Reads the MIR of every body of a lowered module, in the form asked for, flat.
    ///
    /// A body whose check reported a mistake has no MIR ([ADR-0019](../../docs/adr/0019-mir.md))
    /// and is not among the bodies: the diagnostics of the buffer say why, and the reading of
    /// the MIR is what there is to read. A body a `local` declares and a lambda are bodies of
    /// their own, named under the entity that wrote them.
    fn of(driver: &mut Driver, lowered: &Lowered, form: Form) -> Self {
        let mut bodies = Vec::new();

        for body in lowered.bodies() {
            let owner = body.owner().clone();
            let mir = match form {
                Form::Cfg => driver.mir(&owner),
                Form::Ssa => driver.mir_ssa(&owner),
            };

            let Some(mir) = mir else {
                continue;
            };

            let place = ItemLoc::from(owner.item.clone());
            let root = owner
                .item
                .name()
                .map_or_else(|| format!("{:?}", owner.item), ToString::to_string);

            for mir_body in mir.iter() {
                let name = match &mir_body.function {
                    MirFunctionLoc::Entity(_) => entity_name(&place),
                    function => format!("fun {}", function.name(&root, mir_body.name.as_ref()),),
                };
                let range = if mir_body.function.is_lambda() {
                    closure_range(&mir, mir_body.function.lambda())
                } else if matches!(mir_body.function, MirFunctionLoc::Entity(_)) {
                    lowered.item_range(&place).map(covered)
                } else {
                    None
                };

                bodies.push(MirBody::of(mir_body, name, range));
            }
        }

        Self {
            form: form.name(),
            bodies,
        }
    }
}

/// One body of the MIR, as a host reads it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MirBody {
    /// The function the body is: `fun main`, `fun main::aux`, `fun main::<mlkc@lambda-0>`.
    owner: String,

    /// Where the body is written, in bytes, or nothing where it is written nowhere.
    range: Option<[u32; 2]>,

    /// The block the body is entered at, by position.
    entry: u32,

    /// The parameters of the body: one per parameter of the function.
    params: Vec<MirValue>,

    /// The slots of the CFG form, in the order the lowering bound them; empty in the SSA form,
    /// where every value is defined once and no slot is needed.
    locals: Vec<MirLocal>,

    /// The blocks, in the order they are allocated.
    blocks: Vec<MirBlock>,
}

impl MirBody {
    /// Reads one body of the MIR the way a host reads it.
    fn of(body: &mlkc_mir::Body, owner: String, range: Option<[u32; 2]>) -> Self {
        let code = body.code();
        let file = body.module().0;
        let graph = Cfg::of(code);
        let mut blocks = Vec::with_capacity(code.blocks.len());

        for (id, block) in code.blocks.iter() {
            let at = id.index();
            let params = block
                .params
                .iter()
                .copied()
                .map(|value| value_of(code, file, value))
                .collect();
            let stmts = block
                .stmts
                .iter()
                .map(|stmt| {
                    MirLine {
                        text: mir_dump::stmt_text(code, stmt),
                        kind: stmt_kind(stmt),
                        range: span_range(stmt.span, file),
                    }
                })
                .collect();
            let (text, kind, span) = match &block.term {
                Terminator::Goto { span, .. }
                | Terminator::Branch { span, .. }
                | Terminator::Switch { span, .. }
                | Terminator::Return { span, .. }
                | Terminator::Unreachable { span } => {
                    (
                        mir_dump::terminator_text(&block.term),
                        terminator_kind(&block.term),
                        *span,
                    )
                },
            };

            blocks.push(MirBlock {
                label: mir_dump::block_label(id),
                params,
                stmts,
                term: MirLine {
                    text,
                    kind,
                    range: span_range(span, file),
                },
                predecessors: graph.predecessors(at).iter().map(|it| *it as u32).collect(),
                successors: graph.successors(at).iter().map(|it| *it as u32).collect(),
            });
        }

        Self {
            owner,
            range,
            entry: code.entry.index() as u32,
            params: code
                .params
                .iter()
                .copied()
                .map(|value| value_of(code, file, value))
                .collect(),
            locals: code
                .locals
                .iter()
                .map(|(id, local)| {
                    MirLocal {
                        label: mir_dump::local_label(id),
                        name: local.name.as_ref().map(ToString::to_string),
                        ty: local.ty.to_string(),
                        range: span_range(local.span, file),
                    }
                })
                .collect(),
            blocks,
        }
    }
}

/// Where the expression that creates a lambda is written, among the bodies of one HIR body.
fn closure_range(bodies: &Bodies, lambda: Option<LambdaId>) -> Option<[u32; 2]> {
    let lambda = lambda?;

    for body in bodies.iter() {
        let code = body.code();

        for (_, block) in code.blocks.iter() {
            for stmt in &block.stmts {
                if let StmtKind::Assign {
                    rvalue:
                        Rvalue::Closure {
                            lambda: created, ..
                        },
                    ..
                } = &stmt.kind
                    && *created == lambda
                {
                    return span_range(stmt.span, body.module().0);
                }
            }
        }
    }

    None
}

/// One block of a body, as a host reads it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MirBlock {
    /// The label of the block: `b0`.
    label: String,

    /// The block parameters: one per value born at a join in the SSA form, and none in the CFG
    /// form.
    params: Vec<MirValue>,

    /// The statements of the block, in the order they run.
    stmts: Vec<MirLine>,

    /// The terminator the block ends in.
    term: MirLine,

    /// The blocks that come into this one, by position.
    predecessors: Vec<u32>,

    /// The blocks this one goes to, by position, in the order the terminator lists them.
    successors: Vec<u32>,
}

/// One line of a body: a statement, or the terminator of a block.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MirLine {
    /// What the line says: `l0(x) = const 1`, `goto b1(l0)`.
    text: String,

    /// What the line is: `use`, `const`, `call`, or `prim` for a statement, and `goto`,
    /// `branch`, `switch`, `return`, or `unreachable` for the terminator of a block.
    kind: &'static str,

    /// Where the line is written, in bytes, or nothing where it was written nowhere.
    range: Option<[u32; 2]>,
}

/// One value of a body: a parameter, a block parameter, or the value a statement defines.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MirValue {
    /// The label of the value, as the dump reads it: `v0`.
    label: String,

    /// The source type the checker gave it: `Int`, `() -> Unit`.
    ty: String,

    /// Where it is written, in bytes, or nothing where it was written nowhere.
    range: Option<[u32; 2]>,
}

/// One slot of the CFG form: a name a statement writes, and other statements read.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MirLocal {
    /// The label of the slot, as the dump reads it: `l0`.
    label: String,

    /// The name the slot was bound under, if it was bound under one: `x` for `l0(x)`.
    name: Option<String>,

    /// The source type of what the slot holds: `Int`, `() -> Unit`.
    ty: String,

    /// Where the slot is bound, in bytes, or nothing where it is bound nowhere.
    range: Option<[u32; 2]>,
}

/// Reads one value of a body the way a host reads it.
fn value_of(code: CodeRef<'_>, file: FileId, value: ValueId) -> MirValue {
    let data = &code.values[value];

    MirValue {
        label: mir_dump::value_label(value),
        ty: data.ty.to_string(),
        range: span_range(data.span, file),
    }
}

/// Where a span of a body is written, in the bytes a host counts: nothing for a span that was
/// written in another file --- a synthesized node --- since a host marks the buffer it asked
/// about and no other.
fn span_range(span: Span, file: FileId) -> Option<[u32; 2]> {
    (span.file == file).then(|| covered(span.range))
}

/// What a statement of a body is, as a host reads it.
fn stmt_kind(stmt: &Stmt) -> &'static str {
    let StmtKind::Assign { rvalue, .. } = &stmt.kind;

    match rvalue {
        Rvalue::Use(_) => "use",
        Rvalue::Const(_) => "const",
        Rvalue::Call { .. } => "call",
        Rvalue::Closure { .. } => "closure",
        Rvalue::Capture { .. } => "capture",
        Rvalue::Prim { .. } => "prim",
    }
}

/// What a terminator of a block is, as a host reads it.
fn terminator_kind(term: &Terminator) -> &'static str {
    match term {
        Terminator::Goto { .. } => "goto",
        Terminator::Branch { .. } => "branch",
        Terminator::Switch { .. } => "switch",
        Terminator::Return { .. } => "return",
        Terminator::Unreachable { .. } => "unreachable",
    }
}

/// The LIR of the bodies of one module, as a host reads it.
///
/// A body is read the way a person reads it: the blocks in the order they are allocated, the
/// instructions of a block in the order they run, the terminator the block ends in, and the
/// table that says where every value that needs storage lives. Every line and every value
/// carries the range of the buffer it was read from, which is what an editor marks while a
/// pointer is on it.
#[derive(Serialize)]
struct Lir {
    /// The bodies of the module that check clean, in the order it declares them.
    bodies: Vec<LirBody>,
}

impl Lir {
    /// Reads the LIR of every body of a lowered module.
    ///
    /// A body the driver did not lower has no LIR ([ADR-0022](../../docs/adr/0022-wasm-lir.md))
    /// and is not among the bodies: the diagnostics of the buffer say why, and the reading of
    /// the LIR is what there is to read.
    fn of(driver: &mut Driver, lowered: &Lowered) -> Self {
        let mut bodies = Vec::new();

        for body in lowered.bodies() {
            let owner = body.owner().clone();
            let Some(lir) = driver.lir(&owner) else {
                continue;
            };
            let place = ItemLoc::from(owner.item.clone());

            for function in &lir.functions {
                let name = match &function.function {
                    MirFunctionLoc::Entity(_) => entity_name(&place),
                    _ => format!("fun {}", function.name),
                };
                let range = match &function.function {
                    MirFunctionLoc::Entity(_) => lowered.item_range(&place).map(covered),
                    _ => code_range(&function.body, owner.module().0),
                };

                bodies.push(LirBody::of(&function.body, name, range, owner.module().0));
            }
        }

        Self { bodies }
    }
}

/// One body of the LIR, as a host reads it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LirBody {
    /// The function the body is: `fun main`, `fun main::aux`, `fun main::<mlkc@lambda-0>`.
    owner: String,

    /// Where the body is written, in bytes, or nothing where it is written nowhere.
    range: Option<[u32; 2]>,

    /// The block the body is entered at, by position.
    entry: u32,

    /// What the body gives back: `(ref i31)`, `eqref`.
    ret: String,

    /// The parameters of the body: one per parameter of the owner.
    params: Vec<LirValue>,

    /// The locals the body declares after the parameters of the ABI, in order, with the values
    /// that live in each.
    locals: Vec<LirLocal>,

    /// The structured control flow the body is encoded as, or nothing where it is dispatched.
    structure: Option<LirStructure>,

    /// The blocks, in the order they are allocated.
    blocks: Vec<LirBlock>,
}

impl LirBody {
    /// Reads one lifted function the way a host reads it.
    fn of(
        body: &mlkc_lir_wasm::Body,
        owner: String,
        range: Option<[u32; 2]>,
        file: FileId,
    ) -> Self {
        let graph = mlkc_lir_wasm::cfg::Cfg::of(body);
        let parameters = body.params.len() as u32;
        let mut blocks = Vec::with_capacity(body.blocks.len());

        for (id, block) in body.blocks.iter() {
            let at = id.index();
            let params = block
                .params
                .iter()
                .map(|value| lir_value_of(body, *value, file))
                .collect();
            let insts = block
                .insts
                .iter()
                .map(|inst| {
                    LirLine {
                        text: mlkc_lir_wasm::dump::inst_text(body, inst),
                        kind: inst.op.as_str(),
                        range: span_range(inst.span, file),
                    }
                })
                .collect();
            let (text, kind, span) = lir_terminator(&block.term);

            blocks.push(LirBlock {
                label: mlkc_lir_wasm::dump::block_label(id),
                params,
                insts,
                term: LirLine {
                    text,
                    kind,
                    range: span_range(span, file),
                },
                predecessors: graph.predecessors(at).iter().map(|it| *it as u32).collect(),
                successors: graph.successors(at).iter().map(|it| *it as u32).collect(),
            });
        }

        let locals = body
            .locals
            .locals
            .iter()
            .enumerate()
            .map(|(at, ty)| {
                let index = parameters + at as u32;
                let kind = if body.locals.pc == Some(index) {
                    "pc"
                } else if body.locals.scratch == Some(index) {
                    "scratch"
                } else {
                    "value"
                };
                let values = body
                    .values
                    .iter()
                    .filter(|(value, _)| {
                        body.locals.values.get(value.index()).copied().flatten() == Some(index)
                    })
                    .map(|(value, _)| mlkc_lir_wasm::dump::value_label(value))
                    .collect();

                LirLocal {
                    index,
                    ty: ty.to_string(),
                    kind,
                    values,
                }
            })
            .collect();

        Self {
            owner,
            range,
            entry: body.entry.index() as u32,
            ret: body.ret.to_string(),
            params: body
                .params
                .iter()
                .map(|value| lir_value_of(body, *value, file))
                .collect(),
            locals,
            structure: body.structure.as_ref().map(|structure| {
                let mut lines = Vec::new();

                structure_lines(&structure.nodes, 0, file, &mut lines);

                LirStructure { lines }
            }),
            blocks,
        }
    }
}

/// The structured control flow of a body of the LIR, as a host reads it.
///
/// A body the structuring pass structured is encoded as a tree of `block`, `loop`, and `if`
/// frames around the instructions of its blocks; a body it could not structure is dispatched,
/// and the reading is nothing ([ADR-0022](../../docs/adr/0022-wasm-lir.md)).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LirStructure {
    /// The nodes, in the order encoding writes them.
    lines: Vec<LirStructureLine>,
}

/// One node of the structured control flow: a frame, an arm, a leaf, or where control goes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LirStructureLine {
    /// The number of frames the node stands in, which is what a person reads as indentation.
    depth: u32,

    /// What the node is: `block`, `loop`, `if`, `else`, `leaf`, `params`, `br`, `return`, or
    /// `unreachable`.
    kind: &'static str,

    /// What the node says: `block b3`, `leaf b0`, `br 1 -> b3`.
    text: String,

    /// Where the node is written, in bytes, or nothing where it was written nowhere.
    range: Option<[u32; 2]>,
}

/// Writes the nodes of a structure, each at the depth of the frames around it.
fn structure_lines(
    nodes: &[mlkc_lir_wasm::Node],
    depth: u32,
    file: FileId,
    lines: &mut Vec<LirStructureLine>,
) {
    use mlkc_lir_wasm::{Node, dump};

    for node in nodes {
        match node {
            Node::Block { out, body } => {
                lines.push(structure_line(
                    depth,
                    "block",
                    format!("block {}", dump::block_label(*out)),
                    None,
                ));
                structure_lines(body, depth + 1, file, lines);
            },
            Node::Loop { header, body } => {
                lines.push(structure_line(
                    depth,
                    "loop",
                    format!("loop {}", dump::block_label(*header)),
                    None,
                ));
                structure_lines(body, depth + 1, file, lines);
            },
            Node::If {
                cond,
                then_,
                else_,
                span,
            } => {
                lines.push(structure_line(
                    depth,
                    "if",
                    format!("if {}", dump::value_label(*cond)),
                    span_range(*span, file),
                ));
                structure_lines(then_, depth + 1, file, lines);
                lines.push(structure_line(depth, "else", "else".to_owned(), None));
                structure_lines(else_, depth + 1, file, lines);
            },
            Node::Leaf { block } => {
                lines.push(structure_line(
                    depth,
                    "leaf",
                    format!("leaf {}", dump::block_label(*block)),
                    None,
                ));
            },
            Node::Params { target, args, span } => {
                let args = args
                    .iter()
                    .map(|arg| dump::value_label(*arg))
                    .collect::<Vec<_>>()
                    .join(", ");

                lines.push(structure_line(
                    depth,
                    "params",
                    format!("params {}({args})", dump::block_label(*target)),
                    span_range(*span, file),
                ));
            },
            Node::Br {
                depth: relative,
                target,
                span,
            } => {
                lines.push(structure_line(
                    depth,
                    "br",
                    format!("br {relative} -> {}", dump::block_label(*target)),
                    span_range(*span, file),
                ));
            },
            Node::Return { value, span } => {
                lines.push(structure_line(
                    depth,
                    "return",
                    format!("return {}", dump::value_label(*value)),
                    span_range(*span, file),
                ));
            },
            Node::Unreachable { span } => {
                lines.push(structure_line(
                    depth,
                    "unreachable",
                    "unreachable".to_owned(),
                    span_range(*span, file),
                ));
            },
        }
    }
}

/// One line of a structured body, as a host reads it.
fn structure_line(
    depth: u32,
    kind: &'static str,
    text: String,
    range: Option<[u32; 2]>,
) -> LirStructureLine {
    LirStructureLine {
        depth,
        kind,
        text,
        range,
    }
}

/// One block of a body of the LIR, as a host reads it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LirBlock {
    /// The label of the block: `b0`.
    label: String,

    /// The block parameters: where a value coming from several predecessors is born.
    params: Vec<LirValue>,

    /// The instructions of the block, in the order they run.
    insts: Vec<LirLine>,

    /// The terminator the block ends in.
    term: LirLine,

    /// The blocks that come into this one, by position.
    predecessors: Vec<u32>,

    /// The blocks this one goes to, by position, in the order the terminator lists them.
    successors: Vec<u32>,
}

/// One line of a body of the LIR: an instruction, or the terminator of a block.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LirLine {
    /// What the line says: `v1 = i31.get_s v0`, `branch v1 -> b1, b2`.
    text: String,

    /// What the line is: the name of the instruction, or the name of the terminator.
    kind: &'static str,

    /// Where the line is written, in bytes, or nothing where it was written nowhere.
    range: Option<[u32; 2]>,
}

/// One value of a body of the LIR.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LirValue {
    /// The label of the value, as the dump reads it: `v0`.
    label: String,

    /// The type of the machine value: `i32`, `(ref i31)`, `eqref`.
    ty: String,

    /// The WASM local the value lives in, or nothing where it is emitted where it is read.
    local: Option<u32>,

    /// Where it is written, in bytes, or nothing where it was written nowhere.
    range: Option<[u32; 2]>,
}

/// One local of a body of the LIR: what it holds, and what the compiler keeps in it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LirLocal {
    /// The WASM local number: the parameters of the ABI are the ones before these.
    index: u32,

    /// The type of the local: `i32`, `(ref i31)`, `eqref`.
    ty: String,

    /// What the compiler keeps there: `value`, `pc`, or `scratch`.
    kind: &'static str,

    /// The values that live in it, as the dump labels them: `v1`.
    values: Vec<String>,
}

/// Reads one value of a body of the LIR the way a host reads it.
fn lir_value_of(
    body: &mlkc_lir_wasm::Body,
    value: mlkc_lir_wasm::ValueId,
    file: FileId,
) -> LirValue {
    let data = &body.values[value];

    LirValue {
        label: mlkc_lir_wasm::dump::value_label(value),
        ty: data.ty.to_string(),
        local: body.locals.values.get(value.index()).copied().flatten(),
        range: span_range(data.span, file),
    }
}

/// What a terminator of the LIR is, as a host reads it: the text, the kind, and where it is
/// written.
/// Where the code of a lambda is written: what its instructions cover, which is the closest
/// a lifted function comes to a declaration of its own.
fn code_range(body: &mlkc_lir_wasm::Body, file: FileId) -> Option<[u32; 2]> {
    let mut range: Option<[u32; 2]> = None;
    let spans = body.blocks.iter().flat_map(|(_, block)| {
        block
            .insts
            .iter()
            .map(|inst| inst.span)
            .chain(std::iter::once(lir_terminator(&block.term).2))
    });

    for span in spans {
        let Some([from, to]) = span_range(span, file) else {
            continue;
        };

        range = Some(match range {
            Some([start, end]) => [start.min(from), end.max(to)],
            None => [from, to],
        });
    }

    range
}

fn lir_terminator(term: &mlkc_lir_wasm::Terminator) -> (String, &'static str, Span) {
    use mlkc_lir_wasm::Terminator as Lir;
    let span = match term {
        Lir::Goto { span, .. }
        | Lir::Branch { span, .. }
        | Lir::Switch { span, .. }
        | Lir::Return { span, .. }
        | Lir::Unreachable { span } => *span,
    };
    let kind = match term {
        Lir::Goto { .. } => "goto",
        Lir::Branch { .. } => "branch",
        Lir::Switch { .. } => "switch",
        Lir::Return { .. } => "return",
        Lir::Unreachable { .. } => "unreachable",
    };

    (mlkc_lir_wasm::dump::terminator_text(term), kind, span)
}

/// The name of an entity of the surface of a module, as a host reads it: `fun main`.
///
/// The name is the one the entity was declared under, and the kind is what the language
/// declares it with. An entity with no name of its own is read by its place among its kind,
/// which is what the HIR dump reads it by as well.
fn entity_name(item: &ItemLoc) -> String {
    match item.name() {
        Some(name) => format!("{} {name}", item.kind().keyword()),
        None => format!("{item:?}"),
    }
}

/// A diagnostic as an editor reads it: what to say, and where to point.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    /// `error`, `warning`, `note`, or `help`.
    level: String,

    /// `parser` for everything the parser reports.
    category: String,

    /// The two digits of the category, in the order the pipeline runs its stages: `02` for the
    /// parser. A code a person reads is these digits and then the kind's.
    category_code: String,

    /// The code of the kind of mistake within its category.
    code: String,

    /// The message, without the location: the location is in the labels.
    message: String,

    /// Where the diagnostic points, the primary one first.
    labels: Vec<Label>,

    /// What the diagnostic has to add: a hint, an alternative, a note.
    notes: Vec<String>,
}

impl Diagnostic {
    /// Renders a diagnostic for a host, when every label of it is in the same buffer.
    ///
    /// The line and the column are counted from zero and in bytes,
    /// because that is what a byte offset turns into without reading the text again.
    /// An editor that speaks another unit — CodeMirror counts UTF-16 code units —
    /// converts the column of the line it already has.
    fn of(diagnostic: &mlkc_diagnostics::Diagnostic, index: &LineIndex) -> Self {
        Self::rendered(diagnostic, |span| {
            let at = index.line_col(span.range.start());

            (at.line, at.col)
        })
    }

    /// Renders a diagnostic whose labels may be in different buffers.
    ///
    /// Every label says where it is written, and a file the host has no lines for --- a
    /// diagnostic about a file that is gone --- leaves the place at the first line of it.
    fn of_files(
        diagnostic: &mlkc_diagnostics::Diagnostic,
        mut index_of: impl FnMut(FileId) -> Option<Arc<LineIndex>>,
    ) -> Self {
        Self::rendered(diagnostic, |span| {
            let Some(index) = index_of(span.file) else {
                return (0, 0);
            };
            let at = index.line_col(span.range.start());

            (at.line, at.col)
        })
    }

    /// Renders a diagnostic, reading the line and the column of a place from the file it is in.
    fn rendered(
        diagnostic: &mlkc_diagnostics::Diagnostic,
        mut line_col: impl FnMut(Span) -> (u32, u32),
    ) -> Self {
        Self {
            level: diagnostic.level.as_str().to_string(),
            category: diagnostic.category.as_str().to_string(),
            category_code: diagnostic.category.as_code().to_string(),
            code: diagnostic.code.to_string(),
            message: diagnostic.message.clone(),
            labels: diagnostic
                .labels
                .iter()
                .map(|label| {
                    let (line, column) = line_col(label.span);

                    Label {
                        start: u32::from(label.span.range.start()),
                        end: u32::from(label.span.range.end()),
                        line,
                        column,
                        primary: label.primary,
                        message: label.message.clone(),
                    }
                })
                .collect(),
            notes: diagnostic.notes.clone(),
        }
    }
}

/// One place a diagnostic points at.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Label {
    /// The start of the range, in bytes into the file.
    start: u32,

    /// The end of the range, in bytes into the file.
    end: u32,

    /// The line the range starts on, counted from zero.
    line: u32,

    /// The column of that line, counted in bytes from zero.
    column: u32,

    /// Whether this is the place the diagnostic is about, rather than a place it mentions.
    primary: bool,

    /// What the label says: the part of the message that belongs to the place it marks.
    message: String,
}

/// The path of a buffer, as the driver spells paths.
///
/// A browser has no file system, so the path is a name the editor invents,
/// and a virtual one: `/main.mlk` for the buffer the editor holds.
fn path_of(path: &str) -> VfsPath {
    let path = path.strip_prefix('/').unwrap_or(path);

    VfsPath::new_virtual_path(format!("/{path}"))
}

/// A failure that crosses the boundary as an exception.
fn failure(message: &str) -> JsValue {
    JsValue::from_str(message)
}

/// Hands a value to a JavaScript host the way a host reads it.
///
/// The trees of the syntax are maps, and a host reads a map as an object, not as a `Map`;
/// an absent value is `null`, not `undefined`, so that what a host sees
/// does not depend on where it looks.
fn to_js(value: &impl Serialize) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new()
        .serialize_maps_as_objects(true)
        .serialize_missing_as_null(true);

    value
        .serialize(&serializer)
        .map_err(|error| failure(&format!("failed to cross the boundary: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wasm module is compiled for the host in tests,
    /// so everything but the conversions is exercised where it is cheap to inspect.
    #[test]
    fn a_path_becomes_the_path_of_a_virtual_file() {
        assert_eq!(path_of("main.mlk"), path_of("/main.mlk"));
        assert_eq!(
            path_of("/main.mlk"),
            VfsPath::new_virtual_path("/main.mlk".to_string())
        );
    }

    #[test]
    fn the_library_crosses_the_boundary_as_the_files_it_is() {
        let mut driver = WasmDriver::new();
        let files = driver.register_library();
        let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();

        assert_eq!(paths, [
            "/std/core.mlk",
            "/std/prelude.mlk",
            "/std/runtime.mlk"
        ]);
        assert!(
            files[0].text.contains("module project::core"),
            "a host is handed the source, not a path to read"
        );

        // The driver holds them, so a host that shows a file of the library gets its trees.
        for file in files {
            let diagnostics = driver
                .diagnostics_of(&file.path)
                .expect("a file of the library to analyze");

            assert!(
                diagnostics.is_empty(),
                "{}: {}",
                file.path,
                serde_json::to_string(&diagnostics).unwrap_or_default(),
            );
        }
    }

    #[test]
    fn the_project_crosses_the_boundary_as_its_name() {
        assert_eq!(WasmDriver::new().project(), "app");
    }

    #[test]
    fn a_pushed_buffer_crosses_the_boundary_as_trees_and_diagnostics() {
        let mut driver = WasmDriver::new();

        // The names of the language are names the library declares: without it, `Unit` is a
        // name the module cannot resolve.
        driver.register_library();

        assert!(driver.set_text(
            "/main.mlk",
            Some("fun main(): Int =\n    let x = 1 in\n    x\n".to_string())
        ));
        let cst = serde_json::to_value(driver.cst_of("/main.mlk").expect("the file to parse"))
            .expect("the tree to serialize");

        assert_eq!(cst["kind"], "MODULE_ROOT");
        assert!(
            cst["text_range"].is_array(),
            "a host reads the range of a node as a pair of offsets"
        );
        assert!(cst["children"].is_array(), "a host walks the children");
        assert_eq!(
            token_text(&cst, "FUN_KW").as_deref(),
            Some("fun "),
            "a token carries the trivia that follows it"
        );
        assert_eq!(token_text(&cst, "IDENT").as_deref(), Some("main"));
        assert!(
            token_text(&cst, "WHITESPACE").is_none(),
            "trivia is not a child: it belongs to the token it follows"
        );

        let ast = serde_json::to_value(driver.ast_of("/main.mlk").expect("the file to parse"))
            .expect("the tree to serialize");
        let decl = &ast["fields"]["items"]["items"][0];

        assert_eq!(ast["kind"], "ModuleRoot", "a node says what it is");
        assert_eq!(ast["fields"]["items"]["kind"], "ModuleItemList");
        assert_eq!(
            decl["kind"], "FunDecl",
            "a union serializes as the node it holds"
        );
        assert_eq!(
            decl["fields"]["fun_token"]["Ok"]["kind"], "FUN_KW",
            "a required field holds a token, or nothing"
        );
        assert!(
            ast["fields"]["bom_token"].is_null(),
            "an optional field that is missing is null"
        );
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");

        assert!(
            diagnostics.is_empty(),
            "the module is one the language accepts: {}",
            serde_json::to_string(&diagnostics).unwrap_or_default(),
        );
    }

    #[test]
    fn what_the_driver_did_crosses_the_boundary_as_counters() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text(
            "/main.mlk",
            Some("fun main(): Int =\n    let x = 1 in\n    x\n".to_string()),
        );

        let _ = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let taken = Stats::of(&mut driver.driver);

        // The first read of a buffer is work: nothing it needs is held yet. The hits are the
        // consultations of the values the read made and asked for again --- the library is
        // walked more than once by one pull --- and what a first read cannot have is a stale
        // read or a drop.
        let parse = row(&taken, "parse", "/main.mlk");

        assert!(parse.misses > 0, "the parse of a new buffer to be a miss");
        assert_eq!(parse.took, 0.0, "no clock was given: nothing is timed");
        assert!(
            taken
                .rows
                .iter()
                .all(|it| it.stales == 0 && it.dropped == 0),
            "a first read of a buffer to read nothing again",
        );

        // The second read of it is not: what came back is what the driver held.
        let _ = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let taken = Stats::of(&mut driver.driver);
        let parse = row(&taken, "parse", "/main.mlk");

        assert_eq!(parse.misses, 0);
        assert!(parse.hits > 0, "a second read of a buffer to be a hit");
        assert!(
            taken
                .rows
                .iter()
                .all(|it| it.stales == 0 && it.dropped == 0),
            "a second read of a buffer to read nothing again",
        );

        // The check of the buffer is a value of a body, and the row says whose: the name of
        // the body and the module it is written in.
        let checked: Vec<&str> = taken
            .rows
            .iter()
            .filter(|it| it.pass == "check")
            .map(|it| it.unit.as_str())
            .collect();

        assert_eq!(checked, ["/main.mlk: main"]);
    }

    #[test]
    fn a_pass_the_driver_ran_crosses_the_boundary_timed() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text(
            "/main.mlk",
            Some("fun main(): Int =\n    let x = 1 in\n    x\n".to_string()),
        );

        // A host that has a clock of its own hands it over, and what the passes it runs cost is
        // read off it: a parse is not so fast that a machine's clock reports it as nothing.
        driver.driver.set_clock(mlkc_driver::system_clock());

        let _ = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let taken = Stats::of(&mut driver.driver);
        let parse = row(&taken, "parse", "/main.mlk");

        assert!(parse.misses > 0, "the parse of a new buffer to be a miss");
        assert!(
            parse.took > 0.0,
            "the time of the parse to cross the boundary in milliseconds, not {}",
            parse.took,
        );
    }

    /// The counters of one pass for one unit, by the names a host reads them by.
    fn row<'a>(stats: &'a Stats, pass: &str, unit: &str) -> &'a StatsRow {
        stats
            .rows
            .iter()
            .find(|it| it.pass == pass && it.unit == unit)
            .unwrap_or_else(|| panic!("a row for {pass} of {unit}"))
    }

    #[test]
    fn the_hir_reads_as_a_tree_a_host_folds_and_marks_the_buffer_by() {
        /// The part of the source a serialized range covers.
        fn covered<'a>(source: &'a str, range: &serde_json::Value) -> &'a str {
            let at = range.as_array().expect("a range to be a pair");
            let from = at[0].as_u64().expect("a start") as usize;
            let to = at[1].as_u64().expect("an end") as usize;

            &source[from..to]
        }

        let source = "fun main(): Unit =\n    let x = 1 in\n    x\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let hir = driver
            .hir_of("/main.mlk")
            .expect("the file to lower")
            .expect("the module to have a HIR");
        let json = serde_json::to_value(&hir).expect("the HIR to serialize");
        let nodes = json["nodes"].as_array().expect("the HIR to hold lines");

        // The module is read first, and it is a line about nothing a host can mark: the file
        // stands in no project, and the module is called by the name of its file.
        assert_eq!(nodes[0]["text"], "MODULE #0 project::main");
        assert_eq!(nodes[0]["kind"], "module");
        assert!(nodes[0]["range"].is_null());

        // The surface of the module follows, one line per entity and one per thing it is.
        let item = &nodes[1]["children"][0];

        assert_eq!(nodes[1]["text"], "ITEM TREE");
        assert_eq!(nodes[1]["kind"], "section");
        assert_eq!(item["text"], "fun main  @2.0");
        assert_eq!(item["kind"], "item");
        assert_eq!(covered(source, &item["range"]), source.trim_end());
        assert_eq!(item["children"][0]["text"], "visibility: private");
        assert_eq!(
            item["children"][1]["text"], "ret: Unit -> use Unit",
            "the type of the result is a name the prelude brings in"
        );
        assert!(
            item["children"][0]["range"].is_null(),
            "a field of a declaration is about nothing a host marks"
        );

        // And then the names the module is given without writing them: a prelude import is
        // read like any other entity, and being written nowhere, it is a line with no range.
        let prelude = &nodes[1]["children"][2];

        assert_eq!(prelude["text"], "use Unit  prelude");
        assert_eq!(prelude["kind"], "item");
        assert!(prelude["range"].is_null());
        assert_eq!(prelude["children"][0]["text"], "path: std::prelude::Unit");

        // And then the body of the function, as the tree of what it holds.
        let body = &nodes[2];

        assert_eq!(body["text"], "BODY fun main in module #0");
        assert_eq!(body["kind"], "body");
        assert_eq!(covered(source, &body["range"]), source.trim_end());

        let root = &body["children"][0];
        let declaration = &root["children"][0];

        assert_eq!(root["text"], "root");
        assert_eq!(declaration["text"], "expr#2  let pat#0 = expr#0 in expr#1");
        assert_eq!(
            covered(source, &declaration["range"]),
            "let x = 1 in\n    x"
        );

        let pat = &declaration["children"][0];
        let literal = &declaration["children"][1];

        assert_eq!(pat["text"], "pat#0  bind x");
        assert_eq!(covered(source, &pat["range"]), "x");
        assert_eq!(literal["text"], "expr#0  literal 1");
        assert_eq!(covered(source, &literal["range"]), "1");

        // A path says what it names as well as what it is: the `x` of the body is the binding
        // the `let` introduced, and a host marks both ends of that walk.
        let path = &declaration["children"][2]["children"][0];

        assert_eq!(path["text"], "path#0  x -> binding pat#0");
        assert_eq!(covered(source, &path["range"]), "x");
        assert_eq!(covered(source, &path["resolves"]), "x");
        assert_ne!(
            path["range"], path["resolves"],
            "the path and the binding it names are two places in the source"
        );
    }

    #[test]
    fn a_name_written_twice_marks_each_place_it_is_written_at() {
        /// The part of the source a serialized range covers.
        fn covered<'a>(source: &'a str, range: &serde_json::Value) -> &'a str {
            let at = range.as_array().expect("a range to be a pair");
            let from = at[0].as_u64().expect("a start") as usize;
            let to = at[1].as_u64().expect("an end") as usize;

            &source[from..to]
        }

        let source = "fun main(): Int =\n    let x = 42 in\n    x + x\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let hir = driver
            .hir_of("/main.mlk")
            .expect("the file to lower")
            .expect("the module to have a HIR");
        let json = serde_json::to_value(&hir).expect("the HIR to serialize");
        let nodes = json["nodes"].as_array().expect("the HIR to hold lines");

        let declaration = &nodes[2]["children"][0]["children"][0];

        assert_eq!(declaration["text"], "expr#4  let pat#0 = expr#0 in expr#3");

        let sum = &declaration["children"][2];

        assert_eq!(sum["text"], "expr#3  binary expr#1 + expr#2");

        // Both names are one path of the body — the body holds the path once — and the line under
        // each of them marks the name it is written as: what a reader points at is the place
        // they are reading, and not the place the path happens to be written first.
        let left = &sum["children"][0]["children"][0];
        let right = &sum["children"][1]["children"][0];

        assert_eq!(left["text"], "path#0  x -> binding pat#0");
        assert_eq!(right["text"], left["text"]);
        assert_eq!(left["range"], sum["children"][0]["range"]);
        assert_eq!(right["range"], sum["children"][1]["range"]);
        assert_eq!(covered(source, &left["range"]), "x");
        assert_eq!(
            right["range"][0].as_u64().unwrap() as usize,
            source.rfind('x').expect("a second name in the sum"),
            "the second name is marked where it is written"
        );
        // And both of them name the binding the `let` introduced.
        assert_eq!(covered(source, &left["resolves"]), "x");
        assert_eq!(left["resolves"], right["resolves"]);
    }

    #[test]
    fn a_path_of_a_signature_marks_the_type_and_what_it_names() {
        /// The part of the source a serialized range covers.
        fn covered<'a>(source: &'a str, range: &serde_json::Value) -> &'a str {
            let at = range.as_array().expect("a range to be a pair");
            let from = at[0].as_u64().expect("a start") as usize;
            let to = at[1].as_u64().expect("an end") as usize;

            &source[from..to]
        }

        let source = "use std::core::Int\n\nfun main(value: Int): Int =\n    value\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let hir = driver
            .hir_of("/main.mlk")
            .expect("the file to lower")
            .expect("the module to have a HIR");
        let json = serde_json::to_value(&hir).expect("the HIR to serialize");
        let items = json["nodes"][1]["children"]
            .as_array()
            .expect("the items of the module");
        let item = items
            .iter()
            .find(|it| {
                it["text"]
                    .as_str()
                    .is_some_and(|text| text.starts_with("fun main"))
            })
            .expect("the module to declare a function called `main`");
        let param = &item["children"][1];
        let result = &item["children"][2];

        assert_eq!(param["text"], "param: Int -> use Int");
        assert_eq!(
            param["parts"],
            serde_json::json!([
                { "text": "param", "kind": null },
                { "text": ": ", "kind": null },
                { "text": "Int -> use Int", "kind": "path" },
            ]),
            "the type of a parameter is a part of its own, and it reads as a path"
        );
        assert_eq!(covered(source, &param["range"]), "Int");
        assert_eq!(
            covered(source, &param["resolves"]),
            "use std::core::Int",
            "a type a signature writes marks what its name comes from"
        );

        // The two types are written at two places of the same declaration, and each line
        // points at its own.
        assert_eq!(result["text"], "ret: Int -> use Int");
        assert_eq!(covered(source, &result["range"]), "Int");
        assert_ne!(param["range"], result["range"]);
    }

    #[test]
    fn a_path_that_names_an_import_points_at_the_import() {
        /// The part of the source a serialized range covers.
        fn covered<'a>(source: &'a str, range: &serde_json::Value) -> &'a str {
            let at = range.as_array().expect("a range to be a pair");
            let from = at[0].as_u64().expect("a start") as usize;
            let to = at[1].as_u64().expect("an end") as usize;

            &source[from..to]
        }

        let source = "use std::core::Int\n\nfun main(): Int =\n    Int\n";
        let mut driver = WasmDriver::new();

        driver.set_text("/main.mlk", Some(source.to_string()));
        let hir = driver
            .hir_of("/main.mlk")
            .expect("the file to lower")
            .expect("the module to have a HIR");
        let json = serde_json::to_value(&hir).expect("the HIR to serialize");
        let body = &json["nodes"][2];
        let path = &body["children"][0]["children"][0]["children"][0];

        assert_eq!(path["text"], "path#0  Int -> use Int");
        assert_eq!(covered(source, &path["range"]), "Int");
        assert_eq!(
            covered(source, &path["resolves"]),
            "use std::core::Int",
            "a name an import brought in leads to the import"
        );
    }

    #[test]
    fn a_lowering_mistake_crosses_the_boundary_as_a_diagnostic() {
        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text(
            "/main.mlk",
            Some(
                "fun f(): Unit =\n    g()\n\nfun g(): Unit =\n    g()\n\nfun f(): Unit =\n    g()\n"
                    .to_string(),
            ),
        );
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let diagnostics = serde_json::to_value(&diagnostics).expect("the diagnostics to serialize");
        let diagnostics = diagnostics.as_array().expect("diagnostics");

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0]["category"], "lowering");
        assert_eq!(diagnostics[0]["code"], "01");
        assert_eq!(diagnostics[0]["level"], "error");
    }

    #[test]
    fn a_type_mistake_crosses_the_boundary_as_a_diagnostic() {
        const SOURCE: &str = "fun main(): Unit =\n    \"text\"\n";

        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text("/main.mlk", Some(SOURCE.to_string()));
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let json = serde_json::to_value(&diagnostics).expect("the diagnostics to serialize");
        let diagnostics = json.as_array().expect("diagnostics");

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0]["category"], "typechecker");
        assert_eq!(
            diagnostics[0]["categoryCode"], "05",
            "the type checker is the fifth stage of the pipeline"
        );
        assert_eq!(diagnostics[0]["code"], "09");

        // The place of the label is the expression the check reported, and what it says is what
        // an editor shows under the caret.
        let label = &diagnostics[0]["labels"][0];
        let start = SOURCE.find("\"text\"").expect("the literal to be written") as u64;

        assert_eq!(label["primary"], true);
        assert_eq!(label["start"], start);
        assert_eq!(label["end"], start + "\"text\"".len() as u64);
        assert_eq!(label["message"], "expected `Unit`, found `String`");
    }

    #[test]
    fn a_literal_out_of_the_range_of_int_crosses_the_boundary_as_a_mistake() {
        const SOURCE: &str = "fun big(): Int =\n    1099511627776\n";

        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text("/main.mlk", Some(SOURCE.to_string()));
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let json = serde_json::to_value(&diagnostics).expect("the diagnostics to serialize");
        let diagnostics = json.as_array().expect("diagnostics");

        // The range of a literal is the meaning of the type it is written with, so the check is
        // what reports it ([ADR-0018]).
        //
        // [ADR-0018]: ../../docs/adr/0018-values-as-words.md
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0]["category"], "typechecker");
        assert_eq!(diagnostics[0]["categoryCode"], "05");
        assert_eq!(diagnostics[0]["code"], "13");
        assert_eq!(
            diagnostics[0]["message"],
            "the integer literal `1099511627776` is outside the 31-bit range of `Int`"
        );

        // The place of the label is the literal, and what it says is what an editor shows under
        // the caret.
        let label = &diagnostics[0]["labels"][0];
        let start = SOURCE
            .find("1099511627776")
            .expect("the literal to be written") as u64;

        assert_eq!(label["primary"], true);
        assert_eq!(label["start"], start);
        assert_eq!(label["end"], start + "1099511627776".len() as u64);
        assert_eq!(label["message"], "outside the range of `Int`");
    }

    #[test]
    fn the_types_of_a_buffer_cross_the_boundary_as_places_and_types() {
        const SOURCE: &str = "fun main(): Int =\n    let x = 1 in\n    x\n";

        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text("/main.mlk", Some(SOURCE.to_string()));
        let types = driver
            .types_of("/main.mlk")
            .expect("the file to be checked")
            .expect("the module to have types");
        let json = serde_json::to_value(&types).expect("the types to serialize");
        let types = &json;

        // The surface of the module: what its readers read of it.
        assert_eq!(types["surface"][0]["name"], "fun main");
        assert_eq!(types["surface"][0]["ty"], "() -> Int");

        let body = &types["bodies"][0];
        assert_eq!(body["owner"], "fun main");
        let (from, to) = range(body);
        assert_eq!(
            &SOURCE[from..to],
            SOURCE.trim_end(),
            "the declaration of the body is the whole function it was read from",
        );

        // Every node of the body, in the order it is written in: the `let`, the pattern it
        // binds, the value, and the name the body gives back.
        let nodes = body["nodes"].as_array().expect("the nodes of the body");
        let rows: Vec<(&str, &str, &str)> = nodes
            .iter()
            .map(|it| {
                let (from, to) = range(it);

                (
                    it["kind"].as_str().expect("a kind"),
                    it["ty"].as_str().expect("a type"),
                    &SOURCE[from..to],
                )
            })
            .collect();

        assert_eq!(rows.len(), 4, "{rows:?}");
        assert!(rows.iter().all(|it| it.1 == "Int"), "{rows:?}");
        assert_eq!(rows[0].0, "expr");
        assert!(rows[0].2.starts_with("let x = 1"), "{:?}", rows[0]);
        assert_eq!(rows[1].0, "pat");
        assert_eq!(rows[3], ("expr", "Int", "x"));
    }

    #[test]
    fn a_lambda_a_let_generalized_reads_as_the_parameter_and_not_as_a_mistake() {
        // The program an editor painted as a mistake: the type of the lambda is the parameter
        // the `let` generalized over, and no place of it is `{error}`.
        const SOURCE: &str = "fun test(n: Int): Int = let id = fn(x) -> x in id(1)\n";

        let mut driver = WasmDriver::new();
        driver.register_library();

        driver.set_text("/main.mlk", Some(SOURCE.to_string()));
        let types = driver
            .types_of("/main.mlk")
            .expect("the file to be checked")
            .expect("the module to have types");
        let json = serde_json::to_value(&types).expect("the types to serialize");
        let types = &json;

        let nodes = types["bodies"][0]["nodes"]
            .as_array()
            .expect("the nodes of the body");
        let rows: Vec<(&str, &str)> = nodes
            .iter()
            .map(|it| {
                (
                    it["kind"].as_str().expect("a kind"),
                    it["ty"].as_str().expect("a type"),
                )
            })
            .collect();

        for (kind, ty) in &rows {
            assert!(!ty.contains("{error}"), "{kind}: {ty}");
        }

        // The lambda, the parameter it binds, and the body of it are one parameter: the one
        // the `let` generalized over. The call at `Int` is the `Int` it gives back.
        assert!(rows.contains(&("expr", "('0) -> '0")), "{rows:?}");
        assert!(rows.contains(&("pat", "'0")), "{rows:?}");
        assert!(rows.contains(&("expr", "Int")), "{rows:?}");
    }

    #[test]
    fn the_mir_of_a_buffer_crosses_the_boundary_in_both_forms() {
        const SOURCE: &str = "fun main(): Int =\n    let x = 1 in\n    x\n";

        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text("/main.mlk", Some(SOURCE.to_string()));

        // The CFG form reads a body as one block of slots: every expression is lowered into
        // a slot of its own, and the terminator gives one back. Every line says where it was
        // read from, which is what an editor marks the buffer by.
        let cfg = driver
            .mir_of("/main.mlk", Form::Cfg)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&cfg).expect("the MIR to serialize");

        assert_eq!(json["form"], "cfg");

        let body = &json["bodies"][0];

        assert_eq!(body["owner"], "fun main");
        assert_eq!(body["entry"], 0);
        assert!(body["params"].as_array().is_some_and(Vec::is_empty));

        // The slots of the CFG form, in the order the lowering bound them: the result of the
        // `let`, the literal, the name it binds, and the name the body gives back. A slot
        // bound by a pattern says the name it was bound under.
        let locals = body["locals"].as_array().expect("the slots of the body");

        assert_eq!(locals.len(), 4);
        assert_eq!(locals[0]["label"], "l0");
        assert!(locals[0]["name"].is_null());
        assert_eq!(locals[2]["label"], "l2");
        assert_eq!(locals[2]["name"], "x");
        assert_eq!(locals[2]["ty"], "Int");

        let (from, to) = range(&locals[2]);

        assert_eq!(&SOURCE[from..to], "x", "a slot says where it was bound");

        let block = &body["blocks"][0];

        assert_eq!(block["label"], "b0");
        assert!(
            block["params"].as_array().is_some_and(Vec::is_empty),
            "the CFG form has no block parameters",
        );
        assert!(
            block["predecessors"].as_array().is_some_and(Vec::is_empty)
                && block["successors"].as_array().is_some_and(Vec::is_empty),
            "the entry of a body of one block has no edges",
        );

        let lines: Vec<(&str, &str, &str)> = block["stmts"]
            .as_array()
            .expect("the statements of the block")
            .iter()
            .map(|it| {
                let (from, to) = range(it);

                (
                    it["kind"].as_str().expect("a kind"),
                    it["text"].as_str().expect("a line"),
                    &SOURCE[from..to],
                )
            })
            .collect();

        assert_eq!(
            lines,
            [
                ("const", "l1 = const 1", "1"),
                ("use", "l2(x) = use l1", "x"),
                ("use", "l3 = use l2", "x"),
                ("use", "l0 = use l3", "let x = 1 in\n    x"),
            ],
            "the slots of the body, and the places they were read from"
        );

        let term = &block["term"];
        let (from, to) = range(term);

        assert_eq!(
            term["kind"], "return",
            "the body ends by giving a value back"
        );
        assert_eq!(term["text"], "return l0");
        assert_eq!(&SOURCE[from..to], "let x = 1 in\n    x");

        // The SSA form reads the same body with values in place of slots: every value is
        // defined once, and no statement reads a slot.
        let ssa = driver
            .mir_of("/main.mlk", Form::Ssa)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&ssa).expect("the MIR to serialize");

        assert_eq!(json["form"], "ssa");
        assert!(
            json["bodies"][0]["locals"]
                .as_array()
                .is_some_and(Vec::is_empty),
            "the SSA form has no slots",
        );

        let texts: Vec<&str> = json["bodies"][0]["blocks"][0]["stmts"]
            .as_array()
            .expect("the statements of the block")
            .iter()
            .map(|it| it["text"].as_str().expect("a line"))
            .collect();

        assert_eq!(texts, ["v0 = const 1"]);
        assert_eq!(json["bodies"][0]["blocks"][0]["term"]["text"], "return v0");
    }

    #[test]
    fn the_lir_of_a_buffer_crosses_the_boundary_as_instructions_and_locals() {
        const SOURCE: &str = "fun add(a: Int, b: Int): Int = a + b\n";

        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text("/main.mlk", Some(SOURCE.to_string()));

        let lir = driver
            .lir_of("/main.mlk")
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&lir).expect("the LIR to serialize");
        let body = &json["bodies"][0];

        // A body of the LIR is what the back end encodes: the instructions of the target, the
        // value every one of them defines, and where the values that need storage live
        // ([ADR-0022](../../docs/adr/0022-wasm-lir.md)).
        assert_eq!(body["owner"], "fun add");
        assert_eq!(body["ret"], "(ref i31)");
        assert_eq!(body["entry"], 0);

        // A parameter is the local the ABI declared for it: the first two locals.
        let params = body["params"].as_array().expect("the parameters");

        assert_eq!(params.len(), 2);
        assert_eq!(params[0]["ty"], "(ref i31)");
        assert_eq!(params[0]["local"], 0);
        assert_eq!(params[1]["local"], 1);

        // Every value of this body is read once, in the block that defines it, so none of them
        // is given a local of its own: the instructions are emitted where they are read.
        assert!(
            body["locals"].as_array().is_some_and(Vec::is_empty),
            "a value read once to live nowhere: {}",
            body["locals"],
        );

        let block = &body["blocks"][0];
        let lines: Vec<(&str, &str)> = block["insts"]
            .as_array()
            .expect("the instructions of the block")
            .iter()
            .map(|it| {
                (
                    it["kind"].as_str().expect("a kind"),
                    it["text"].as_str().expect("a line"),
                )
            })
            .collect();

        assert_eq!(
            lines,
            [
                ("i31.get_s", "v2 = i31.get_s v0"),
                ("i31.get_s", "v3 = i31.get_s v1"),
                ("i32.add", "v4 = i32.add v2, v3"),
                ("ref.i31", "v5 = ref.i31 v4"),
            ],
            "the operator as the unboxed instruction and the box of its result",
        );

        let (from, to) = range(&block["term"]);

        assert_eq!(block["term"]["kind"], "return");
        assert_eq!(block["term"]["text"], "return v5");
        assert_eq!(&SOURCE[from..to], "a + b");
    }

    #[test]
    fn a_lambda_crosses_the_boundary_as_a_body_of_its_own() {
        // A lambda is lifted into a function of the module, flat, and named under the entity that
        // wrote it; the expression that creates it is what its header points at.
        const SOURCE: &str = "fun one(): Int =\n    let add = fn(x: Int) -> x + 1 in\n    add(1)\n\nfun two(): Int =\n    let add = fn(x: Int) -> fn(y: Int) -> x + y in\n    add(1)(2)\n";

        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text("/main.mlk", Some(SOURCE.to_string()));

        let cfg = driver
            .mir_of("/main.mlk", Form::Cfg)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&cfg).expect("the MIR to serialize");

        let one = &json["bodies"][0];
        let add = &json["bodies"][1];

        assert_eq!(one["owner"], "fun one");
        assert_eq!(add["owner"], "fun one::<mlkc@lambda-0>");
        assert_eq!(add["entry"], 0);

        let params = add["params"]
            .as_array()
            .expect("the parameters of the lambda");

        assert_eq!(params.len(), 1, "a lambda of one parameter");
        assert_eq!(params[0]["ty"], "Int");

        // The header of a lambda stands for the expression that wrote it: pointing at it asks
        // the editor to mark that code.
        let (from, to) = range(add);

        assert_eq!(&SOURCE[from..to], "fn(x: Int) -> x + 1");

        // Every lambda is a body of the module of its own: the outer one comes first, because
        // the lowering meets it first, and the lambda it gives back follows it.
        let outer = &json["bodies"][3];
        let inner = &json["bodies"][4];

        assert_eq!(outer["owner"], "fun two::<mlkc@lambda-0>");
        assert_eq!(inner["owner"], "fun two::<mlkc@lambda-1>");

        let (from, to) = range(outer);

        assert_eq!(&SOURCE[from..to], "fn(x: Int) -> fn(y: Int) -> x + y");

        let (from, to) = range(inner);

        assert_eq!(&SOURCE[from..to], "fn(y: Int) -> x + y");

        // The SSA form reads the same bodies flat, with every value defined once: a lambda body
        // is a body like any other.
        let ssa = driver
            .mir_of("/main.mlk", Form::Ssa)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&ssa).expect("the MIR to serialize");
        let add = &json["bodies"][1];

        assert_eq!(add["owner"], "fun one::<mlkc@lambda-0>");
        assert!(
            add["locals"].as_array().is_some_and(Vec::is_empty),
            "the SSA form has no slots",
        );
        assert_eq!(json["bodies"][4]["owner"], "fun two::<mlkc@lambda-1>");
    }

    #[test]
    fn a_function_declared_in_a_local_crosses_the_boundary_as_a_body_of_its_own() {
        const SOURCE: &str =
            "fun main(): Int =\n    local fun double(x: Int): Int = x * 2 in\n    double(21)\n";

        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text("/main.mlk", Some(SOURCE.to_string()));

        // The MIR reads the function as a body of its own, named under the entity that declares
        // it, with the parameters its signature wrote.
        let cfg = driver
            .mir_of("/main.mlk", Form::Cfg)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&cfg).expect("the MIR to serialize");
        let double = &json["bodies"][1];
        let params = double["params"].as_array().expect("the parameters");

        assert_eq!(double["owner"], "fun main::double");
        assert_eq!(params.len(), 1, "the function to declare one parameter");
        assert_eq!(params[0]["ty"], "Int");
        assert!(
            !double["blocks"].as_array().expect("the blocks").is_empty(),
            "the function to be a body of its own",
        );

        // The LIR reads the lifted function under the same name.
        let lir = driver
            .lir_of("/main.mlk")
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&lir).expect("the LIR to serialize");
        let double = &json["bodies"][1];

        assert_eq!(double["owner"], "fun main::double");
        assert!(
            !double["blocks"].as_array().expect("the blocks").is_empty(),
            "the lifted function to be a body of its own",
        );
    }

    #[test]
    fn a_lifted_lambda_crosses_the_boundary_as_the_body_it_is() {
        const SOURCE: &str = "fun main(): Int =\n    let add = fn(x: Int) -> fn(y: Int) -> x + y in\n    add(1)(2)\n";

        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text("/main.mlk", Some(SOURCE.to_string()));

        let lir = driver
            .lir_of("/main.mlk")
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&lir).expect("the LIR to serialize");

        // A lifted lambda is a function of the module, flat under the entity that wrote it.
        let add = &json["bodies"][1];

        assert_eq!(add["owner"], "fun main::<mlkc@lambda-0>");
        assert_eq!(add["ret"], "eqref");
        assert!(
            !add["blocks"].as_array().expect("the blocks").is_empty(),
            "the lambda to be a body of its own",
        );

        // The lambda the outer one gives back is a lifted function with it.
        let inner = &json["bodies"][2];

        assert_eq!(inner["owner"], "fun main::<mlkc@lambda-1>");
        assert_eq!(
            inner["params"].as_array().expect("the parameters").len(),
            2,
            "a lifted lambda is entered with its environment, and the parameter it declared",
        );

        // Where the lambda is written is what its lifted body covers: the instructions of
        // a lambda are read from the expression it computes.
        let (from, to) = range(inner);

        assert!(
            SOURCE[from..to].contains("fn(y: Int)") || SOURCE[from..to].contains("x + y"),
            "the lambda to point at what it was read from: {:?}",
            &SOURCE[from..to],
        );
    }

    #[test]
    fn the_mir_of_a_choice_crosses_the_boundary_with_the_blocks_it_branches_into() {
        const SOURCE: &str =
            "fun pick(flag: Bool): Int =\n    if flag then 1 elif flag then 2 else 3\n";

        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text("/main.mlk", Some(SOURCE.to_string()));

        // The CFG form reads the choice as the blocks it branches into: the entry evaluates
        // the first condition, the block of every arm that fails evaluates the next one, and
        // the blocks the arms are meet where the value of the expression is read.
        let cfg = driver
            .mir_of("/main.mlk", Form::Cfg)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&cfg).expect("the MIR to serialize");
        let blocks = json["bodies"][0]["blocks"]
            .as_array()
            .expect("the blocks of the body");

        assert_eq!(blocks.len(), 6);
        assert_eq!(json["bodies"][0]["entry"], 0);
        assert_eq!(blocks[0]["term"]["kind"], "branch");
        assert_eq!(
            blocks[0]["successors"],
            serde_json::json!([1, 2]),
            "the entry goes to the first arm and to the condition after it",
        );
        assert_eq!(blocks[2]["term"]["kind"], "branch");
        assert_eq!(blocks[2]["successors"], serde_json::json!([3, 4]));
        assert_eq!(
            blocks[5]["predecessors"],
            serde_json::json!([1, 3, 4]),
            "the block the arms meet in is entered from every arm",
        );
        assert_eq!(blocks[5]["term"]["kind"], "return");

        // The branch of the entry is about the condition it read, and a host marks it by the
        // place the condition is written at.
        let (from, to) = range(&blocks[0]["term"]);

        assert_eq!(&SOURCE[from..to], "flag");
        assert_eq!(blocks[0]["term"]["text"], "branch l2 -> b1, b2");

        // The SSA form gives the value the arms agree on a parameter of the block they meet in,
        // and every arm passes its own value to it.
        let ssa = driver
            .mir_of("/main.mlk", Form::Ssa)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&ssa).expect("the MIR to serialize");
        let blocks = json["bodies"][0]["blocks"]
            .as_array()
            .expect("the blocks of the body");
        let join = &blocks[5];

        assert_eq!(
            join["params"].as_array().map(Vec::len),
            Some(1),
            "the value the arms agree on is born at the join",
        );
        assert_eq!(join["params"][0]["ty"], "Int");

        for arm in [1, 3, 4] {
            let text = blocks[arm]["term"]["text"]
                .as_str()
                .expect("a terminator line");

            assert!(
                text.starts_with("goto b5("),
                "every arm passes its value to the join: {text}",
            );
        }

        // The value born at the join is the expression the arms are: a host marks the whole
        // choice where the value is written.
        let (from, to) = range(&join["params"][0]);

        assert_eq!(&SOURCE[from..to], "if flag then 1 elif flag then 2 else 3");
    }

    #[test]
    fn the_mir_of_a_choice_without_an_else_crosses_the_boundary_as_a_unit() {
        const SOURCE: &str = "fun log(flag: Bool): Unit =\n    if flag then\n        log(flag)\n";

        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text("/main.mlk", Some(SOURCE.to_string()));

        // The choice selects no value when it has no `else`: the block control falls into writes
        // the unit it is, and a host reads it as the arm that is not written.
        let cfg = driver
            .mir_of("/main.mlk", Form::Cfg)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&cfg).expect("the MIR to serialize");
        let blocks = json["bodies"][0]["blocks"]
            .as_array()
            .expect("the blocks of the body");

        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0]["successors"], serde_json::json!([1, 2]));
        assert!(
            blocks[2]["stmts"]
                .as_array()
                .expect("the statements of the fall-through")
                .iter()
                .any(|it| it["text"] == "l1 = const unit"),
            "the arm that is not written writes the unit: {}",
            blocks[2]["stmts"],
        );
        assert_eq!(blocks[3]["predecessors"], serde_json::json!([1, 2]));

        // The SSA form gives the unit a parameter of the block the arms meet in like any other
        // value, and the arm that is not written passes it like any other arm.
        let ssa = driver
            .mir_of("/main.mlk", Form::Ssa)
            .expect("the file to be read")
            .expect("the module to lower");
        let json = serde_json::to_value(&ssa).expect("the MIR to serialize");
        let blocks = json["bodies"][0]["blocks"]
            .as_array()
            .expect("the blocks of the body");

        assert_eq!(blocks[3]["params"].as_array().map(Vec::len), Some(1));
        assert_eq!(blocks[3]["params"][0]["ty"], "Unit");
        assert!(
            blocks[2]["term"]["text"]
                .as_str()
                .expect("a terminator line")
                .starts_with("goto b3("),
            "the arm that is not written passes the unit to the join: {}",
            blocks[2]["term"]["text"],
        );
    }

    #[test]
    fn a_body_of_a_join_crosses_the_boundary_with_its_parameters_and_edges() {
        use mlkc_hir_def::{
            Attributes, BodyEntityLoc, BodyLoc, EntityData, FunctionData, ItemSyntaxLoc,
            ItemTreeBuilder, Name, PathRoot, PlainPath, PlainPathId, Signature, Visibility,
        };
        use mlkc_mir::{
            Block, BlockTarget, BodyBuilder, Const, LocalData, Operand, Place, Rvalue, Stmt,
            StmtKind, ValueData,
        };
        use mlkc_vfs::FileId;

        // A body the way the SSA form leaves a join: the entry branches into two arms, each of
        // them passes a value to a third block, and the third block is entered through
        // a parameter. No source lowers to one yet --- the language has no conditional yet ---
        // and what this checks is the reading of one, not the construction.
        let mut tree = ItemTreeBuilder::new(
            ModuleId(FileId::from_raw(0)),
            PlainPathId::new(PlainPath::from_root(PathRoot::Project, [Name::new("join")])),
        );

        tree.declare(
            Some(Name::new("join")),
            EntityData::Function(FunctionData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
                signature: Signature::default(),
            }),
            ItemSyntaxLoc::root().child(0),
        );

        let tree = tree.finish();
        let (item, _) = tree.entities().next().expect("the function to be declared");
        let ItemLoc::Function(function) = item else {
            panic!("the entity is a function");
        };
        let owner = BodyEntityLoc {
            module: tree.module(),
            item: BodyLoc::Function(function),
        };
        let span = Span::dummy();
        let mut builder = BodyBuilder::new(MirFunctionLoc::Entity(owner), Ty::Error);
        let cond = builder.param(ValueData {
            span,
            ty: Ty::Error,
        });
        let join = builder.value(ValueData {
            span,
            ty: Ty::Error,
        });
        let carried = builder.local(LocalData {
            span,
            name: Some(Name::new("x")),
            ty: Ty::Error,
        });
        let entry = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable { span },
        });
        let left = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable { span },
        });
        let right = builder.block(Block {
            params: Vec::new(),
            stmts: Vec::new(),
            term: Terminator::Unreachable { span },
        });
        let done = builder.block(Block {
            params: vec![join],
            stmts: Vec::new(),
            term: Terminator::Return {
                value: Operand::Value(join),
                span,
            },
        });

        *builder.block_mut(entry) = Block {
            params: Vec::new(),
            stmts: vec![Stmt {
                kind: StmtKind::Assign {
                    place: Place::Local(carried),
                    rvalue: Rvalue::Const(Const::Int(1)),
                },
                span,
            }],
            term: Terminator::Branch {
                cond: Operand::Value(cond),
                then_: BlockTarget {
                    block: left,
                    args: Vec::new(),
                },
                else_: BlockTarget {
                    block: right,
                    args: Vec::new(),
                },
                span,
            },
        };

        for arm in [left, right] {
            *builder.block_mut(arm) = Block {
                params: Vec::new(),
                stmts: Vec::new(),
                term: Terminator::Goto {
                    target: BlockTarget {
                        block: done,
                        args: vec![Operand::Local(carried)],
                    },
                    span,
                },
            };
        }

        let body = builder.finish(entry);
        let json = serde_json::to_value(MirBody::of(&body, "fun join".to_owned(), None))
            .expect("the body to serialize");

        assert_eq!(json["entry"], 0);
        assert_eq!(json["params"][0]["label"], "v0");
        assert_eq!(json["locals"][0]["label"], "l0");
        assert_eq!(json["locals"][0]["name"], "x");
        assert_eq!(json["blocks"][0]["term"]["kind"], "branch");
        assert_eq!(json["blocks"][1]["term"]["text"], "goto b3(l0)");
        assert_eq!(json["blocks"][3]["params"][0]["label"], "v1");
        assert_eq!(json["blocks"][3]["term"]["text"], "return v1");
        assert_eq!(
            json["blocks"][3]["predecessors"],
            serde_json::json!([1, 2]),
            "the join is entered from both arms",
        );
        assert_eq!(json["blocks"][0]["successors"], serde_json::json!([1, 2]));
        assert_eq!(json["blocks"][1]["successors"], serde_json::json!([3]));
        assert!(
            json["blocks"][0]["stmts"][0]["range"].is_null(),
            "a synthesized node was written nowhere",
        );
    }

    #[test]
    fn the_wat_of_a_buffer_crosses_the_boundary_as_text() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text(
            "/main.mlk",
            Some("#[entry]\npub fun main(): Unit =\n    print-int(21 + 21)\n".to_string()),
        );

        let wat = driver
            .wat_of("/main.mlk")
            .expect("the file to be read")
            .expect("the module to compile");

        assert!(
            wat.diagnostics.is_empty(),
            "the module compiles cleanly: {}",
            serde_json::to_string(&wat.diagnostics).unwrap_or_default(),
        );
        assert!(wat.text.contains("(module"), "{}", wat.text);
        assert!(
            wat.text.contains("app::main"),
            "a module names itself in its name section",
        );
        assert!(
            wat.text.contains("print-int"),
            "what the module imports is what it calls",
        );
        assert!(
            wat.text.contains("ref.i31"),
            "an immediate is a word the module makes",
        );
    }

    #[test]
    fn a_buffer_that_does_not_check_has_no_wat() {
        let mut driver = WasmDriver::new();
        driver.register_library();

        // `+` reads two `Int`s, and the checker reports the boolean: a module that is not whole
        // is not one the back end assembles.
        driver.set_text(
            "/main.mlk",
            Some("fun main(): Int =\n    true + 1\n".to_string()),
        );

        assert!(
            driver
                .wat_of("/main.mlk")
                .expect("the file to be read")
                .is_none(),
        );
    }

    #[test]
    fn a_run_carries_the_modules_a_host_wires_together() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver
            .set_options("source-map", "none")
            .expect("the options to be known");
        driver.set_text(
            "/main.mlk",
            Some(
                "#[entry]\npub fun main(): Unit =\n    let flag = small(3) in\n    let noted = \
                 print-bool(flag) in\n    print-int(21 + 21)\n\npub fun small(value: Int): Bool =\n    \
                 value < 5\n"
                    .to_string(),
            ),
        );

        let run = driver.program_of().expect("the program to describe");
        let entry = run.entry.as_ref().expect("the program to declare an entry");

        assert_eq!(entry.module, "app::main");
        assert_eq!(entry.name, "main");
        assert_eq!(run.manifest.project, "app");
        assert_eq!(
            run.manifest
                .entry
                .as_ref()
                .map(|it| (it.module.as_str(), it.name.as_str())),
            Some(("app::main", "main")),
        );
        assert!(run.problems.is_empty(), "{:?}", run.problems);
        assert!(
            run.diagnostics.is_empty(),
            "{}",
            serde_json::to_string(&run.diagnostics).unwrap_or_default(),
        );
        assert_eq!(
            run.modules.last().map(|module| module.name.as_str()),
            Some("app::main"),
            "a provider comes before the modules that import it",
        );
        assert!(
            run.modules
                .iter()
                .any(|module| module.name == "std::runtime"),
            "the library is part of the program",
        );

        // The debug information of a browser is the source map, and the editor runs under the
        // map alone ([`DebugInfo::SourceMap`]): the program carries it, and no table of DWARF
        // is there for an engine to prefer to it.
        for module in &run.modules {
            let names: Vec<String> = custom_sections(&module.bytes)
                .into_iter()
                .map(|section| section.name)
                .collect();

            assert!(
                names.iter().all(|name| !is_debug_section(name)),
                "the run of {} to carry no DWARF: {names:?}",
                module.name,
            );
        }

        let main = run.modules.last().expect("the program to have a module");
        let names: Vec<String> = custom_sections(&main.bytes)
            .into_iter()
            .map(|section| section.name)
            .collect();

        assert!(
            names.iter().any(|name| name == "sourceMappingURL"),
            "the debug information of a browser to be there: {names:?}",
        );

        // The host of the test: the numbers the program prints, in the order it prints them.
        let mut config = wasmtime::Config::new();

        config.wasm_gc(true);
        config.wasm_function_references(true);

        let engine = wasmtime::Engine::new(&config).expect("the engine to start");
        let mut store = wasmtime::Store::new(&engine, Vec::<String>::new());
        let mut linker = wasmtime::Linker::new(&engine);

        linker
            .func_wrap(
                "host",
                "print-int",
                |mut caller: wasmtime::Caller<'_, Vec<String>>, value: i32| -> i32 {
                    caller.data_mut().push(value.to_string());

                    0
                },
            )
            .expect("the host function to be defined");
        linker
            .func_wrap(
                "host",
                "print-bool",
                |mut caller: wasmtime::Caller<'_, Vec<String>>, value: i32| -> i32 {
                    caller.data_mut().push((value != 0).to_string());

                    0
                },
            )
            .expect("the host function to be defined");

        // The host functions come first, and what they export is what an extern resolves to:
        // a module of the program imports an extern under the canonical name of the module that
        // declares it, and a host stands in for that module.
        let host = wasmtime::Module::new(&engine, &run.host).expect("the host module to compile");
        let host = linker
            .instantiate(&mut store, &host)
            .expect("the host module to instantiate");
        let mut externs = std::collections::BTreeSet::new();

        for module in &run.modules {
            for import in &module.imports {
                if import.external {
                    externs.insert((import.module.clone(), import.name.clone()));
                }
            }
        }

        for (module, name) in externs {
            let function = host
                .get_func(&mut store, &name)
                .expect("the host function to be exported");

            linker
                .define(&store, &module, &name, function)
                .expect("the extern to be defined");
        }

        let mut instances = std::collections::BTreeMap::new();

        for module in &run.modules {
            let compiled = wasmtime::Module::new(&engine, &module.bytes)
                .unwrap_or_else(|error| panic!("`{}` to compile: {error}", module.name));
            let instance = linker
                .instantiate(&mut store, &compiled)
                .unwrap_or_else(|error| panic!("`{}` to instantiate: {error}", module.name));

            for export in &module.exports {
                let function = instance
                    .get_func(&mut store, &export.name)
                    .unwrap_or_else(|| {
                        panic!("`{}::{}` to be a function", module.name, export.name)
                    });

                linker
                    .define(&store, &module.name, &export.name, function)
                    .expect("the export to be defined");
            }

            instances.insert(module.name.clone(), instance);
        }

        let instance = instances.get(&entry.module).expect("the entry module");
        let main = instance
            .get_func(&mut store, &entry.name)
            .expect("the entry to be exported");
        let mut results = [wasmtime::Val::AnyRef(None)];

        main.call(&mut store, &[], &mut results)
            .expect("the entry point to run");

        assert_eq!(store.into_data(), ["true", "42"]);
    }

    #[test]
    fn a_program_without_an_entry_is_a_problem_a_host_reads() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text(
            "/main.mlk",
            Some("pub fun twice(value: Int): Int =\n    value * 2\n".to_string()),
        );

        let run = driver.program_of().expect("the program to describe");

        assert!(run.entry.is_none());
        assert_eq!(run.problems, ["the program declares no `#[entry]`"]);
    }

    #[test]
    fn an_extern_no_host_function_implements_is_a_problem() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text(
            "/main.mlk",
            Some(
                "#[entry]\npub fun main(): Unit =\n    putchar(65)\n\n#[extern]\npub fun \
                 putchar(value: Int): Unit\n"
                    .to_string(),
            ),
        );

        let run = driver.program_of().expect("the program to describe");

        assert_eq!(run.problems, [
            "the host does not implement the extern `app::main::putchar`"
        ],);
    }

    #[test]
    fn a_file_of_the_library_is_not_moved_into_the_project_of_the_page() {
        let mut driver = WasmDriver::new();
        let files = driver.register_library();
        let core = files
            .iter()
            .find(|file| file.path == "/std/core.mlk")
            .expect("the core module");

        // An editor shows a file of the library as a buffer, and reading it again is a push to
        // the driver: the file is the compiler's, and the project it is read under is the one
        // the compiler put it in ([ADR-0015]).
        //
        // [adr-0015]: ../../docs/adr/0015-standard-library.md
        driver.set_text("/std/core.mlk", Some(core.text.to_string()));

        let file = driver
            .driver
            .file_id(&path_of("/std/core.mlk"))
            .expect("the file to have an id");
        let project = driver
            .driver
            .project_graph()
            .project_of(ModuleId(file))
            .cloned();

        assert_eq!(project, Some(ProjectId::new(mlkc_stdlib::PROJECT)));
    }

    #[test]
    fn a_buffer_that_is_gone_leaves_the_project_of_the_page() {
        let mut driver = WasmDriver::new();
        driver.register_library();
        driver.set_text(
            "/main.mlk",
            Some("#[entry]\npub fun main(): Unit =\n    print-int(1)\n".to_string()),
        );
        driver.set_text(
            "/gone.mlk",
            Some("pub fun gone(): Int =\n    1\n".to_string()),
        );

        // The buffer is a module of the project while it is there.
        let run = driver.program_of().expect("the program to describe");

        assert!(
            run.modules.iter().any(|module| module.name == "app::gone"),
            "the program holds the module of the buffer: {:?}",
            run.modules.iter().map(|it| &it.name).collect::<Vec<_>>(),
        );

        // A host that drops the buffer says so, and the module leaves the project with it.
        assert!(driver.set_text("/gone.mlk", None));

        let _ = Stats::of(&mut driver.driver);
        let run = driver.program_of().expect("the program to describe");

        assert!(
            !run.modules.iter().any(|module| module.name == "app::gone"),
            "the program does not hold a module whose file is gone",
        );

        // Nothing of the project reads the file that is gone: a host that forgot to say so
        // would pay for a parse that is not there on every look.
        let taken = Stats::of(&mut driver.driver);

        assert!(
            taken.rows.iter().all(|row| row.unit != "/gone.mlk"),
            "the project is not read over a file that is not there: {:?}",
            taken
                .rows
                .iter()
                .map(|it| (&it.pass, &it.unit))
                .collect::<Vec<_>>(),
        );

        // And the buffer written again is a module of the project again.
        driver.set_text(
            "/gone.mlk",
            Some("pub fun gone(): Int =\n    2\n".to_string()),
        );

        let run = driver.program_of().expect("the program to describe");

        assert!(run.modules.iter().any(|module| module.name == "app::gone"));
    }

    /// The place a serialized row points at, in the bytes a host counts.
    fn range(row: &serde_json::Value) -> (usize, usize) {
        let at = row["range"].as_array().expect("a range to be a pair");

        (
            at[0].as_u64().expect("a start") as usize,
            at[1].as_u64().expect("an end") as usize,
        )
    }

    /// The text of the first token of a kind, wherever in a serialized tree it sits.
    fn token_text(node: &serde_json::Value, kind: &str) -> Option<String> {
        if node["kind"] == kind {
            return node["text"].as_str().map(str::to_string);
        }

        node["children"]
            .as_array()?
            .iter()
            .find_map(|child| token_text(child, kind))
    }

    #[test]
    fn a_mistake_becomes_a_diagnostic_a_host_can_point_at() {
        let mut driver = WasmDriver::new();

        driver.set_text(
            "/main.mlk",
            Some("fun main(): Unit =\n    let x = 1\n".to_string()),
        );
        let diagnostics = driver
            .diagnostics_of("/main.mlk")
            .expect("the file to be diagnosed");
        let diagnostic = diagnostics
            .into_iter()
            .next()
            .expect("the parse to report something");

        assert_eq!(diagnostic.level, "error");
        assert_eq!(diagnostic.category, "parser");
        assert_eq!(
            diagnostic.category_code, "02",
            "the parser is the second stage of the pipeline"
        );
        assert_eq!(diagnostic.code, "01");

        let label = diagnostic
            .labels
            .first()
            .expect("a label to point somewhere");

        assert!(label.primary);
        assert_eq!(
            label.line, 2,
            "the missing `in` is reported at the end of the file"
        );
        assert!(label.end >= label.start);
    }

    #[test]
    fn a_node_the_grammar_has_no_room_for_serializes_what_it_holds() {
        let mut driver = WasmDriver::new();

        driver.set_text(
            "/main.mlk",
            Some("fun main(): Unit =\n    x\nabc\n".to_string()),
        );
        let ast = driver
            .ast_of("/main.mlk")
            .expect("the file to parse")
            .expect("the module root to be found");
        let json = serde_json::to_value(&ast).expect("the tree to serialize");
        let bogus = bogus_node(&json).expect("the mistake to leave a node of no kind");

        assert!(
            bogus["items"].is_array(),
            "what such a node holds is a list of elements, as it is for any list"
        );
        assert!(
            bogus.get("syntax").is_none(),
            "the syntax a node wraps is its own business, not something a host reads"
        );
    }

    /// The first value of a serialized tree whose kind is one the grammar has no room for.
    fn bogus_node(value: &serde_json::Value) -> Option<&serde_json::Value> {
        if let Some(kind) = value["kind"].as_str()
            && kind.starts_with("Bogus")
        {
            return Some(value);
        }

        match value {
            serde_json::Value::Array(items) => items.iter().find_map(bogus_node),
            serde_json::Value::Object(entries) => entries.values().find_map(bogus_node),
            _ => None,
        }
    }
}
