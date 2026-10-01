//! The module a back end assembles: its functions, and the WASM module they become.
//!
//! One MLK module becomes one WASM module ([ADR-0020][adr-0020], [ADR-0021][adr-0021]).
//! [`ModuleMir`] is what `assemble_module` reads: the checked bodies and their MIR, the
//! signatures their owners have, and the classes of the language.
//!
//! Assembly is deterministic: indices are assigned in the order the module declares its
//! entities, so two compilations of the same module agree byte for byte. The layout is a value
//! of its own ([`layout`]), which is what `emit_function` is handed as well --- a function
//! body embeds the index of every callee --- so the emitter and the assembler cannot disagree
//! about an index.
//!
//! [adr-0020]: ../../docs/adr/0020-wasm-backend.md
//! [adr-0021]: ../../docs/adr/0021-translation-units.md

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{BodyEntityLoc, BodyLoc, EntityLoc, FunctionLoc, Name};
use mlkc_hir_ty::{Builtins, Ty};
use mlkc_mir::Body as MirBody;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, FunctionSection, IndirectNameMap, NameMap, NameSection,
    RefType, TypeSection, ValType,
};

use crate::emit::{CodegenDiag, FuncArtifact};

/// The signature of a body: one word per parameter, and one word back.
///
/// The types are the checker's, and they say what a word refines to at the boundary of the
/// function; the WASM signature itself is all words ([ADR-0018][adr-0018]).
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnSignature {
    /// The parameters, in the order the function takes them.
    pub params: Vec<Ty>,
    /// What the function gives back.
    pub ret: Ty,
}

/// One function of a module, as the back end reads it.
#[derive(Debug, Clone)]
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

/// One module: the functions it declares, and the classes of the language.
#[derive(Debug, Clone)]
pub struct ModuleMir {
    /// The name of the module, which names it in the `name` section and in a stack trace.
    pub name: String,
    /// The classes of the language, which say what a type refines to.
    pub builtins: Builtins,
    /// The functions, in the order the module declares them.
    pub functions: Vec<ModuleFunction>,
}

/// How the entities of a module are numbered in the WASM module it becomes.
///
/// The layout is computed from the module alone, so the emitter and the assembler both derive
/// the same indices, and a function body can embed the index of the function it calls.
#[derive(Debug, Clone)]
pub struct ModuleLayout {
    builtins: Builtins,
    functions: Vec<FnSignature>,
    by_entity: BTreeMap<EntityLoc<FunctionLoc>, u32>,
}

impl ModuleLayout {
    /// The classes of the language.
    pub fn builtins(&self) -> &Builtins {
        &self.builtins
    }

    /// The index of the function an entity names, if the module declares it.
    pub fn function_index(&self, entity: &EntityLoc<FunctionLoc>) -> Option<u32> {
        self.by_entity.get(entity).copied()
    }

    /// The signature of the function an entity names, if the module declares it.
    pub fn signature_of(&self, entity: &EntityLoc<FunctionLoc>) -> Option<&FnSignature> {
        let index = self.by_entity.get(entity)?;

        self.functions.get(*index as usize)
    }

    /// The number of functions of the module.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Whether the module declares no function.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

/// Numbers the functions of a module: one index per function, in declaration order.
pub fn layout(module: &ModuleMir) -> ModuleLayout {
    let mut functions = Vec::with_capacity(module.functions.len());
    let mut by_entity = BTreeMap::new();

    for (index, function) in module.functions.iter().enumerate() {
        let index = index as u32;

        functions.push(function.signature.clone());

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
        functions,
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

/// The WASM module the assembler wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmModule {
    /// The bytes of the module.
    pub bytes: Vec<u8>,
    /// What the module exports, in the order it exports it; the linker reads this rather than
    /// parsing what the assembler just wrote.
    pub exports: Vec<WasmExport>,
}

/// One export of a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExport {
    /// The name the export is under.
    pub name: String,
    /// The index of the exported function.
    pub function: u32,
}

/// Assembles the WASM module of `module` from the artifacts of its functions.
///
/// `functions` are the artifacts of every function of the module, in declaration order; the
/// `debug` level says how much debug information the module carries.
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

    // Every function of the module has the same WASM signature shape: one `eqref` per
    // parameter and one back. One type per arity is all the section needs, and the arities are
    // sorted so that an assignment of indices follows from the module alone.
    let mut arities: Vec<usize> = module
        .functions
        .iter()
        .map(|function| function.signature.params.len())
        .collect();

    arities.sort_unstable();
    arities.dedup();

    let type_index = |arity: usize| -> u32 {
        arities
            .binary_search(&arity)
            .expect("the arity of a function of the module to be in the table") as u32
    };

    let word = ValType::Ref(RefType::EQREF);
    let mut types = TypeSection::new();

    for arity in &arities {
        types.ty().function((0..*arity).map(|_| word), [word]);
    }

    let mut function_section = FunctionSection::new();

    for function in &module.functions {
        function_section.function(type_index(function.signature.params.len()));
    }

    let mut exports = ExportSection::new();
    let mut export_table = Vec::new();

    for (index, function) in module.functions.iter().enumerate() {
        if function.exported {
            exports.export(&function.name, ExportKind::Func, index as u32);
            export_table.push(WasmExport {
                name: function.name.clone(),
                function: index as u32,
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
    wasm.section(&function_section);
    wasm.section(&exports);
    wasm.section(&code);

    let mut names = NameSection::new();
    names.module(&module.name);

    let mut function_names = NameMap::new();

    for (index, function) in module.functions.iter().enumerate() {
        function_names.append(index as u32, &function.name);
    }

    names.functions(&function_names);

    let mut local_names = IndirectNameMap::new();

    for (index, function) in module.functions.iter().enumerate() {
        let mut names = NameMap::new();

        // The ABI parameters are locals of every function, and so are the locals the values
        // live in; a name is written for each of them where the source has one.
        for (parameter, name) in function.param_names.iter().enumerate() {
            if let Some(name) = name {
                names.append(parameter as u32, name.as_str());
            }
        }

        if let Some(artifact) = functions.get(index) {
            for value in &artifact.debug {
                if let Some(name) = &value.name {
                    names.append(value.local, name.as_str());
                }
            }
        }

        local_names.append(index as u32, &names);
    }

    names.locals(&local_names);

    wasm.section(&names);

    (
        WasmModule {
            bytes: wasm.finish(),
            exports: export_table,
        },
        Vec::new(),
    )
}
