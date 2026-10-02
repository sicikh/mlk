//! The module a back end assembles: its imports, its functions, and the WASM module they become.
//!
//! One MLK module becomes one WASM module ([ADR-0020][adr-0020], [ADR-0021][adr-0021]).
//! [`ModuleMir`] is what `assemble_module` reads: the checked bodies and their MIR, the
//! signatures their owners have, the functions they call and do not declare, and the classes of
//! the language.
//!
//! A function the module does not declare is an import: a function of another module, or one
//! declared `#[extern]` and implemented by the host. Its WASM index comes before every local
//! function, which is what the function index space of a module is, and that is why a layout is
//! a value of its own ([`layout`]): the emitter and the assembler both read it, so a body embeds
//! the same index the assembler numbered the function with.
//!
//! Assembly is deterministic: imports keep the order the module lists them in, functions the
//! order the module declares them, and two compilations of one module agree byte for byte.
//!
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md
//! [adr-0021]: ../../docs/adr/0021-translation-units.md

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{BodyEntityLoc, BodyLoc, EntityLoc, FunctionLoc, Name};
use mlkc_hir_ty::{Builtins, Ty};
use mlkc_lir_wasm::Body as LirBody;
use mlkc_mir::Body as MirBody;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, FunctionSection, ImportSection,
    IndirectNameMap, NameMap, NameSection, TypeSection,
};

use crate::{
    emit::{CodegenDiag, FuncArtifact, FunctionCtx, emit_function},
    refine::{self, AbiType},
};

/// The signature of a body: one parameter per parameter of the owner, and one result.
///
/// The types are the checker's, and the WASM signature is the shape they give ([`FnShape`]):
/// an immediate crosses as an `(ref i31)` and every other type as a word, `eqref`
/// ([ADR-0018][adr-0018]).
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnSignature {
    /// The parameters, in the order the function takes them.
    pub params: Vec<Ty>,
    /// What the function gives back.
    pub ret: Ty,
}

impl FnSignature {
    /// The shape the signature crosses the ABI as ([`FnShape`]).
    pub fn shape(&self, builtins: &Builtins) -> FnShape {
        FnShape {
            params: self
                .params
                .iter()
                .map(|ty| refine::of_ty(ty, builtins).abi())
                .collect(),
            ret: self.ret_shape(builtins),
        }
    }

    /// What the result crosses the ABI as.
    pub fn ret_shape(&self, builtins: &Builtins) -> AbiType {
        refine::of_ty(&self.ret, builtins).abi()
    }
}

/// What a signature crosses the ABI as: what every parameter and the result are ([`AbiType`]).
///
/// An immediate is an `(ref i31)`, which a value of an immediate type already is, and every
/// other type is a word, an `eqref` ([ADR-0018][adr-0018]). Two signatures of one shape share
/// a type in the type section of the module.
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnShape {
    /// What every parameter crosses as, in the order the function takes them.
    pub params: Vec<AbiType>,
    /// What the result crosses as.
    pub ret: AbiType,
}

/// A function the module needs and does not declare.
///
/// It is one of another module of the program, which the linker finds by the canonical path of
/// that module, or one declared `#[extern]` anywhere, which the host provides under the same
/// name ([ADR-0021][adr-0021]).
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleImport {
    /// The entity the import is of; it is what a body of this module calls.
    pub entity: EntityLoc<FunctionLoc>,
    /// The canonical path of the module the function belongs to.
    pub module: String,
    /// The name of the function inside its module.
    pub name: String,
    /// What the function takes and gives back.
    pub signature: FnSignature,
    /// Whether the function is declared `#[extern]`.
    ///
    /// An external function is implemented outside the program, so the import is what a host
    /// provides and not a dependency on another module ([ADR-0021]).
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    pub external: bool,
}

/// One function of a module, as the back end reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleFunction {
    /// The entity that owns the body.
    pub owner: BodyEntityLoc,
    /// The name the function is called by; the `name` section and the export use it.
    pub name: String,
    /// What the function takes and gives back.
    pub signature: FnSignature,
    /// The name every parameter was declared under, in order; `None` for a pattern with no name.
    pub param_names: Vec<Option<Name>>,
    /// Whether the function is exported from the module.
    pub exported: bool,
    /// The SSA body of the function.
    pub body: Arc<MirBody>,
}

/// One module: the functions it imports, the ones it declares, and the classes of the language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleMir {
    /// The canonical path of the module, which names it in the `name` section, in an import of
    /// another module, and in a stack trace.
    pub name: String,
    /// The classes of the language, which say what a type refines to.
    pub builtins: Builtins,
    /// The functions the module imports, in the order their indices are assigned.
    pub imports: Vec<ModuleImport>,
    /// The functions the module declares, in the order the module declares them.
    pub functions: Vec<ModuleFunction>,
}

/// How the functions of a module are numbered in the WASM module it becomes.
///
/// The layout is computed from the module alone, so the emitter and the assembler both derive
/// the same indices, and a function body can embed the index of the function it calls. The
/// index space is the one of WASM: the imports first, in the order the module lists them, and
/// the functions the module declares after them, in declaration order.
#[derive(Debug, Clone)]
pub struct ModuleLayout {
    builtins: Builtins,
    /// One signature per function of the index space.
    signatures: Vec<FnSignature>,
    /// How many of the signatures are imports; the rest are the module's own functions.
    imports: usize,
    by_entity: BTreeMap<EntityLoc<FunctionLoc>, u32>,
}

impl ModuleLayout {
    /// The classes of the language.
    pub fn builtins(&self) -> &Builtins {
        &self.builtins
    }

    /// The index of the function an entity names, if the module declares or imports it.
    pub fn function_index(&self, entity: &EntityLoc<FunctionLoc>) -> Option<u32> {
        self.by_entity.get(entity).copied()
    }

    /// The signature of the function an entity names, if the module declares or imports it.
    pub fn signature_of(&self, entity: &EntityLoc<FunctionLoc>) -> Option<&FnSignature> {
        let index = self.by_entity.get(entity)?;

        self.signatures.get(*index as usize)
    }

    /// How many functions of the index space are imports.
    pub fn imported(&self) -> usize {
        self.imports
    }

    /// The number of functions in the index space, imports included.
    pub fn len(&self) -> usize {
        self.signatures.len()
    }

    /// Whether the module imports and declares no function.
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }
}

/// Numbers the functions of a module: the imports first, then the functions it declares.
pub fn layout(module: &ModuleMir) -> ModuleLayout {
    let mut signatures = Vec::with_capacity(module.imports.len() + module.functions.len());
    let mut by_entity = BTreeMap::new();

    for import in &module.imports {
        let index = signatures.len() as u32;

        signatures.push(import.signature.clone());
        by_entity.insert(import.entity.clone(), index);
    }

    for function in &module.functions {
        let index = signatures.len() as u32;

        signatures.push(function.signature.clone());

        // A body is a function's when the entity that owns it is one; a constant's body has no
        // function to be called as.
        if let BodyLoc::Function(loc) = &function.owner.item {
            by_entity.insert(
                EntityLoc {
                    module: function.owner.module,
                    item: loc.clone(),
                },
                index,
            );
        }
    }

    ModuleLayout {
        builtins: module.builtins.clone(),
        signatures,
        imports: module.imports.len(),
        by_entity,
    }
}

/// How much debug information `assemble_module` is asked for ([ADR-0020][adr-0020]).
///
/// The level is configuration, which is an input, so the debug tables of a module are
/// absent-or-equal for equal input ([ADR-0008][adr-0008]). The DWARF custom sections are the
/// next milestone; what every level emits for now is the `name` section, which is what makes a
/// stack trace readable without DWARF.
///
/// [adr-0008]: ../../docs/adr/0008-compiler-driver.md
/// [adr-0020]: ../../docs/adr/0020-wasm-backend.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugLevel {
    /// No debug information.
    #[default]
    None,
    /// Line tables only.
    Lines,
    /// Line tables, functions, parameters, and locals.
    Full,
}

/// One function a module imports: what a linker resolves ([ADR-0021][adr-0021]).
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    /// The canonical path of the module the function belongs to.
    pub module: String,
    /// The name of the function inside its module.
    pub name: String,
    /// How many words the function takes; it gives back one.
    pub arity: u32,
    /// Whether the function is declared `#[extern]`.
    ///
    /// An external function is implemented outside the program, so the import is what a host
    /// provides and not a dependency on another module ([ADR-0021]): a linker reads this to
    /// know whether to look for a provider among the modules or among its own functions.
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    pub external: bool,
}

/// One function a module offers: what a linker resolves an import to ([ADR-0021][adr-0021]).
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportDecl {
    /// The canonical path of the module that offers the function.
    pub module: String,
    /// The name of the function inside its module.
    pub name: String,
    /// How many words the function takes; it gives back one.
    pub arity: u32,
}

/// The WASM module the assembler wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmModule {
    /// The bytes of the module.
    pub bytes: Vec<u8>,
    /// What it needs from other modules and from the host; the linker reads this rather than
    /// parsing what the assembler just wrote.
    pub imports: Vec<ImportDecl>,
    /// What it offers, in the order it offers it; the linker reads this likewise.
    pub exports: Vec<ExportDecl>,
}

/// Compiles every function of a module, and assembles the module they become.
///
/// This is the whole of the back end for one module: [`emit_function`] per function, in the
/// order the module declares them, and [`assemble_module`] over the artifacts. It is what the
/// driver's link stage and a host that shows one module both read.
///
/// `lirs` are the lowered bodies of the module's functions, in declaration order, as the
/// driver pulled them ([ADR-0022][adr-0022]).
///
/// # Panics
///
/// Panics when the number of bodies is not the number of functions of the module, which is a
/// mistake of the caller and not of the program.
///
/// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
pub fn compile_module(module: &ModuleMir, lirs: &[Arc<LirBody>]) -> (WasmModule, Vec<CodegenDiag>) {
    assert_eq!(
        lirs.len(),
        module.functions.len(),
        "the back end is handed one body per function of the module",
    );

    let layout = layout(module);
    let mut artifacts = Vec::new();
    let mut diagnostics = Vec::new();

    for (function, lir) in module.functions.iter().zip(lirs) {
        let ctx = FunctionCtx {
            name: &function.name,
            signature: &function.signature,
            param_names: &function.param_names,
            layout: &layout,
        };
        let (artifact, reports) = emit_function(lir, &ctx);

        artifacts.push(artifact);
        diagnostics.extend(reports);
    }

    let (wasm, reports) = assemble_module(module, &artifacts, DebugLevel::Full);

    diagnostics.extend(reports);

    (wasm, diagnostics)
}

/// Assembles the WASM module of `module` from the artifacts of its functions.
///
/// `functions` are the artifacts of every function the module declares, in declaration order;
/// the `debug` level says how much debug information the module carries.
///
/// # Panics
///
/// Panics when the number of artifacts is not the number of functions of the module, which is
/// a mistake of the caller and not of the program: the driver emits one artifact per body it
/// lowered and nothing else.
pub fn assemble_module(
    module: &ModuleMir,
    functions: &[FuncArtifact],
    debug: DebugLevel,
) -> (WasmModule, Vec<CodegenDiag>) {
    assert_eq!(
        functions.len(),
        module.functions.len(),
        "the assembler is handed one artifact per function of the module",
    );

    // The DWARF custom sections read the level; the `name` section is a part of the module and
    // is written at every level.
    let _ = debug;

    // Every function of the module has the WASM type its signature's shape gives it: an
    // immediate is a `(ref i31)` and every other type is a word, an `eqref`. One type per
    // shape is all the section needs, and the shapes are interned in the order the module
    // meets them, so an assignment of indices follows from the module alone.
    let builtins = &module.builtins;
    let mut shapes: Vec<FnShape> = Vec::new();
    let mut import_types = Vec::with_capacity(module.imports.len());

    for import in &module.imports {
        import_types.push(intern_shape(&mut shapes, import.signature.shape(builtins)));
    }

    let mut function_types = Vec::with_capacity(module.functions.len());

    for function in &module.functions {
        function_types.push(intern_shape(
            &mut shapes,
            function.signature.shape(builtins),
        ));
    }

    let mut types = TypeSection::new();

    for shape in &shapes {
        let ret = shape.ret.val_type();

        types
            .ty()
            .function(shape.params.iter().map(|it| it.val_type()), [ret]);
    }

    let mut import_section = ImportSection::new();
    let mut import_table = Vec::with_capacity(module.imports.len());

    for (index, import) in module.imports.iter().enumerate() {
        import_section.import(
            &import.module,
            &import.name,
            EntityType::Function(import_types[index]),
        );
        import_table.push(ImportDecl {
            module: import.module.clone(),
            name: import.name.clone(),
            arity: import.signature.params.len() as u32,
            external: import.external,
        });
    }

    // The functions the module declares take the indices after the imports; nothing else is
    // numbered in the function index space.
    let offset = module.imports.len() as u32;
    let mut function_section = FunctionSection::new();

    for type_index in &function_types {
        function_section.function(*type_index);
    }

    let mut exports = ExportSection::new();
    let mut export_table = Vec::new();

    for (index, function) in module.functions.iter().enumerate() {
        if function.exported {
            let index = offset + index as u32;

            exports.export(&function.name, ExportKind::Func, index);
            export_table.push(ExportDecl {
                module: module.name.clone(),
                name: function.name.clone(),
                arity: function.signature.params.len() as u32,
            });
        }
    }

    let mut code = CodeSection::new();

    for artifact in functions {
        // `CodeSection::raw` writes the size prefix of the entry itself; the artifact holds the
        // body without it, because the layout is what the debug tables measure.
        code.raw(&artifact.body);
    }

    let mut wasm = wasm_encoder::Module::new();

    wasm.section(&types);
    wasm.section(&import_section);
    wasm.section(&function_section);
    wasm.section(&exports);
    wasm.section(&code);

    let mut names = NameSection::new();
    names.module(&module.name);

    let mut function_names = NameMap::new();

    for (index, import) in module.imports.iter().enumerate() {
        function_names.append(index as u32, &import.name);
    }

    for (index, function) in module.functions.iter().enumerate() {
        function_names.append(offset + index as u32, &function.name);
    }

    names.functions(&function_names);

    let mut local_names = IndirectNameMap::new();

    for (index, function) in module.functions.iter().enumerate() {
        let mut names = NameMap::new();

        // The ABI parameters are locals of every function, and so are the locals the values
        // live in; a name is written for each of them where the source has one. An imported
        // function has no body and no locals of its own.
        for (parameter, name) in function.param_names.iter().enumerate() {
            if let Some(name) = name {
                names.append(parameter as u32, name.as_str());
            }
        }

        if let Some(artifact) = functions.get(index) {
            for value in &artifact.debug {
                let Some(name) = &value.name else {
                    continue;
                };

                // A value that is a parameter lives in the local of that parameter, and the
                // loop above named it from the function; the artifact's name for it is the
                // same one.
                if value.local < function.param_names.len() as u32 {
                    continue;
                }

                names.append(value.local, name.as_str());
            }
        }

        local_names.append(offset + index as u32, &names);
    }

    names.locals(&local_names);

    wasm.section(&names);

    (
        WasmModule {
            bytes: wasm.finish(),
            imports: import_table,
            exports: export_table,
        },
        Vec::new(),
    )
}

/// The index a shape has in the type section, interning it when it is not there yet.
///
/// The shapes are in the order the module meets them, which is a function of the module alone,
/// so two assemblies of one module number the types the same way.
fn intern_shape(shapes: &mut Vec<FnShape>, shape: FnShape) -> u32 {
    match shapes.iter().position(|it| *it == shape) {
        Some(index) => index as u32,
        None => {
            shapes.push(shape);

            (shapes.len() - 1) as u32
        },
    }
}
