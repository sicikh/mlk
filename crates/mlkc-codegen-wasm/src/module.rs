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

use std::{borrow::Cow, collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{BodyEntityLoc, BodyLoc, EntityLoc, FunctionLoc, ItemLocLike, Name};
use mlkc_hir_ty::{Builtins, Ty};
use mlkc_la_arena::Arena;
use mlkc_lir_wasm::Body as LirBody;
use mlkc_mir::{Body as MirBody, CodeRef, LambdaData, LambdaId, Rvalue, StmtKind};
use wasm_encoder::{
    CodeSection, CompositeInnerType, CompositeType, ElementSection, Elements, EntityType,
    ExportKind, ExportSection, FieldType, FuncType, FunctionSection, HeapType, ImportSection,
    IndirectNameMap, NameMap, NameSection, RefType, StorageType, StructType, SubType, TypeSection,
    ValType,
};

use crate::{
    dwarf::{self, Sources},
    emit::{CodegenDiag, FuncArtifact, FunctionCtx, emit_function},
    refine::{self, AbiType},
    sourcemap,
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

/// The `$fn` and `$closure` types of one shape ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosureTypes {
    /// The type of the code: `$fn(shape)`, which takes the closure first.
    pub fn_type: u32,
    /// The type of the closure: `$closure(shape)`, a struct whose one field is the code.
    pub closure: u32,
}

/// What the layout knows about one lambda ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LambdaPlan {
    /// The index of the lifted function in the function index space.
    pub index: u32,
    /// What the lambda takes and gives back, without the environment.
    pub signature: FnSignature,
    /// The shapes of its captures, in the order they are stored.
    pub captures: Vec<AbiType>,
    /// The `$fn` and `$closure` types of its shape.
    pub types: ClosureTypes,
    /// The type of its environment, where it captured something.
    pub env: Option<u32>,
}

impl LambdaPlan {
    /// Whether the lambda captured something, and so has an environment of its own.
    pub fn captures(&self) -> bool {
        !self.captures.is_empty()
    }
}

/// What a function of the index space is ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionKey {
    /// A function an entity declares.
    Entity(EntityLoc<FunctionLoc>),
    /// A lambda written in a body.
    Lambda {
        /// The entity whose body wrote it; the lambda is an entry of that body's arena.
        owner: BodyEntityLoc,
        /// The lambda, by its place in the arena.
        lambda: LambdaId,
    },
}

/// The LIR of one function and of the lambdas it wrote ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredFunction {
    /// What the function is.
    pub key: FunctionKey,
    /// The LIR of the function.
    pub body: LirBody,
    /// The LIR of the lambdas written in it, in the order the body creates them.
    pub lambdas: Vec<LoweredFunction>,
}

/// One function the back end emitted: the artifact and what names it ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedFunction {
    /// What the function is.
    pub key: FunctionKey,
    /// The name of the function, for the `name` section, an export, and a stack trace.
    pub name: String,
    /// Whether the module exports it; a lambda is never exported.
    pub exported: bool,
    /// How many parameters the function's declared signature takes.
    pub arity: u32,
    /// The name every parameter was declared under, in order.
    pub param_names: Vec<Option<Name>>,
    /// The WASM type of the function: a plain function type for an entity, the `$fn` of its
    /// shape for a lambda.
    pub type_index: u32,
    /// The artifact of its body.
    pub artifact: FuncArtifact,
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

/// How the functions of a module are numbered in the WASM module it becomes ([ADR-0026]).
///
/// The layout is computed from the module alone, so the emitter and the assembler both derive
/// the same indices, and a function body can embed the index of the function it calls and the
/// index of the lambda a closure wraps. The index space is the one of WASM: the imports first,
/// in the order the module lists them, and then the functions the module declares in
/// declaration order, each immediately followed by its lambdas, depth first.
///
/// The layout also plans the type section: the plain function types, the shape groups
/// `{$fn, $closure}` of every function shape a closure is made of, and the environment type of
/// every capture shape a lambda creates.
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone)]
pub struct ModuleLayout {
    builtins: Builtins,
    /// One declared signature per function of the index space, in index order.
    signatures: Vec<FnSignature>,
    /// How many of the signatures are imports; the rest are the module's own functions.
    imports: usize,
    by_entity: BTreeMap<EntityLoc<FunctionLoc>, u32>,
    by_lambda: BTreeMap<(BodyEntityLoc, LambdaId), LambdaPlan>,
    /// The plain function types of the type section, in index order.
    plain: Vec<FnShape>,
    /// The shape groups of the type section, in index order; the group of `shapes[i]` is at
    /// `shape_base() + 2 * i`.
    shapes: Vec<FnShape>,
    /// The environment groups of the type section, in index order; the group of `envs[i]` is at
    /// `env_base() + i`.
    envs: Vec<(FnShape, Vec<AbiType>)>,
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

    /// What the layout knows about a lambda a body wrote.
    pub fn lambda(&self, owner: &BodyEntityLoc, lambda: LambdaId) -> Option<&LambdaPlan> {
        self.by_lambda.get(&(owner.clone(), lambda))
    }

    /// The type of a plain function shape, the one the imports and the entity bodies use.
    pub fn plain_type(&self, shape: &FnShape) -> Option<u32> {
        self.plain
            .iter()
            .position(|it| it == shape)
            .map(|at| at as u32)
    }

    /// The `$fn` and `$closure` types of a function shape, where a closure of it exists.
    pub fn closure_types(&self, shape: &FnShape) -> Option<ClosureTypes> {
        let at = self.shapes.iter().position(|it| it == shape)?;

        Some(ClosureTypes {
            fn_type: self.shape_base() + 2 * at as u32,
            closure: self.shape_base() + 2 * at as u32 + 1,
        })
    }

    /// The type of the environment of a capture shape, where one exists.
    pub fn env_type(&self, shape: &FnShape, captures: &[AbiType]) -> Option<u32> {
        let at = self
            .envs
            .iter()
            .position(|(it, fields)| it == shape && fields.as_slice() == captures)?;

        Some(self.env_base() + at as u32)
    }

    /// Whether the concrete type `ty` upcasts to the closure type `closure` without a cast:
    /// the closure type itself, or an environment of it.
    pub fn upcasts(&self, ty: u32, closure: u32) -> bool {
        if ty == closure {
            return true;
        }

        let Some(at) = ty.checked_sub(self.env_base()) else {
            return false;
        };

        self.envs
            .get(at as usize)
            .and_then(|(shape, _)| self.closure_types(shape))
            .is_some_and(|types| types.closure == closure)
    }

    /// The plain function types, in index order.
    pub fn plain_shapes(&self) -> &[FnShape] {
        &self.plain
    }

    /// The shapes whose groups the type section declares, in index order.
    pub fn shape_groups(&self) -> &[FnShape] {
        &self.shapes
    }

    /// The environments whose groups the type section declares, in index order.
    pub fn env_groups(&self) -> &[(FnShape, Vec<AbiType>)] {
        &self.envs
    }

    /// The index the shape groups begin at.
    pub fn shape_base(&self) -> u32 {
        self.plain.len() as u32
    }

    /// The index the environment groups begin at.
    pub fn env_base(&self) -> u32 {
        self.shape_base() + 2 * self.shapes.len() as u32
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

/// Numbers the functions of a module, and plans the types their closures use.
///
/// The functions are the imports, then the entity functions in declaration order, each followed
/// by its lambdas, depth first ([ADR-0026]). The types are planned in the order the module
/// meets them: the plain function types, the shape groups of the closure family, and the
/// environments; the shapes and the environments are deduplicated, so two lambdas that capture
/// the same things share one type.
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
pub fn layout(module: &ModuleMir) -> ModuleLayout {
    let builtins = module.builtins.clone();
    let mut planner = Types::default();
    let mut signatures = Vec::with_capacity(module.imports.len() + module.functions.len());
    let mut by_entity = BTreeMap::new();
    let mut pending = Vec::new();

    for import in &module.imports {
        let index = signatures.len() as u32;

        signatures.push(import.signature.clone());
        planner.plain(&import.signature.shape(&builtins));
        by_entity.insert(import.entity.clone(), index);
    }

    for function in &module.functions {
        let index = signatures.len() as u32;

        signatures.push(function.signature.clone());
        planner.plain(&function.signature.shape(&builtins));

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

        for lambda in lambda_order(function.body.code(), &function.body.lambdas) {
            let data = &function.body.lambdas[lambda];
            let signature = signature_of_lambda(&data.ty);
            let shape = signature.shape(&builtins);
            let captures: Vec<AbiType> = data
                .captures
                .iter()
                .map(|capture| refine::of_ty(&capture.ty, &builtins).abi())
                .collect();
            let at = planner.shape(&shape);
            let env = (!captures.is_empty()).then(|| planner.env(&shape, &captures));
            let index = signatures.len() as u32;

            signatures.push(signature.clone());
            pending.push(((function.owner.clone(), lambda), PendingLambda {
                index,
                signature,
                captures,
                at,
                env,
            }));
        }

        // A call through a closure needs the shape's types whether or not a lambda of this
        // module made the closure.
        planner.calls(function.body.code(), &builtins);

        for (_, lambda) in function.body.lambdas.iter() {
            planner.calls(lambda.code(), &builtins);
        }
    }

    let plain = planner.plain;
    let shapes = planner.shapes;
    let envs = planner.envs;
    let shape_base = plain.len() as u32;
    let env_base = shape_base + 2 * shapes.len() as u32;
    let mut by_lambda = BTreeMap::new();

    for (key, lambda) in pending {
        let types = ClosureTypes {
            fn_type: shape_base + 2 * lambda.at as u32,
            closure: shape_base + 2 * lambda.at as u32 + 1,
        };

        by_lambda.insert(key, LambdaPlan {
            index: lambda.index,
            signature: lambda.signature,
            captures: lambda.captures,
            types,
            env: lambda.env.map(|env| env_base + env as u32),
        });
    }

    ModuleLayout {
        builtins,
        signatures,
        imports: module.imports.len(),
        by_entity,
        by_lambda,
        plain,
        shapes,
        envs,
    }
}

/// A lambda the layout has to number, before the type indices are known.
struct PendingLambda {
    index: u32,
    signature: FnSignature,
    captures: Vec<AbiType>,
    at: usize,
    env: Option<usize>,
}

/// The types the module's closures use, interned in the order the module meets them.
#[derive(Default)]
struct Types {
    plain: Vec<FnShape>,
    shapes: Vec<FnShape>,
    envs: Vec<(FnShape, Vec<AbiType>)>,
}

impl Types {
    /// The plain function type of a shape.
    fn plain(&mut self, shape: &FnShape) {
        if !self.plain.contains(shape) {
            self.plain.push(shape.clone());
        }
    }

    /// The shape group of a shape, by its place among the groups.
    fn shape(&mut self, shape: &FnShape) -> usize {
        match self.shapes.iter().position(|it| it == shape) {
            Some(at) => at,
            None => {
                self.shapes.push(shape.clone());

                self.shapes.len() - 1
            },
        }
    }

    /// The environment group of a capture shape, by its place among the groups.
    fn env(&mut self, shape: &FnShape, captures: &[AbiType]) -> usize {
        match self
            .envs
            .iter()
            .position(|(it, fields)| it == shape && fields.as_slice() == captures)
        {
            Some(at) => at,
            None => {
                self.envs.push((shape.clone(), captures.to_vec()));

                self.envs.len() - 1
            },
        }
    }

    /// The shapes of the calls through a closure in one piece of code.
    fn calls(&mut self, code: CodeRef<'_>, builtins: &Builtins) {
        for (_, block) in code.blocks.iter() {
            for stmt in &block.stmts {
                let StmtKind::Assign { rvalue, .. } = &stmt.kind;
                let Rvalue::Call {
                    callee: mlkc_mir::Callee::Indirect(operand),
                    ..
                } = rvalue
                else {
                    continue;
                };
                let Some(shape) = operand_shape(operand, code, builtins) else {
                    continue;
                };

                self.shape(&shape);
            }
        }
    }
}

/// The declared signature of a lambda: the function type the checker gave it.
///
/// # Panics
///
/// Panics when the type is not a function type, which is a gap of the check: a lambda the
/// check accepted is a function.
fn signature_of_lambda(ty: &Ty) -> FnSignature {
    let Ty::Fn { params, ret } = ty else {
        panic!("the lowering met a lambda the checker did not give a function type: {ty}");
    };

    FnSignature {
        params: params.clone(),
        ret: ret.as_ref().clone(),
    }
}

/// The ABI shape of the checked type of the operand a closure is called through.
fn operand_shape(
    operand: &mlkc_mir::Operand,
    code: CodeRef<'_>,
    builtins: &Builtins,
) -> Option<FnShape> {
    let mlkc_mir::Operand::Value(value) = operand else {
        return None;
    };

    refine::closure_shape_of_ty(&code.values[*value].ty, builtins)
}

/// The lambdas a body wrote, depth first, in the order its code creates them.
///
/// A lambda is created by exactly one `Rvalue::Closure`, in the code that wrote the expression;
/// the walk reads every block, because a lambda created in a block no path reaches is still a
/// lambda of the body, and the order is the order the blocks and the statements list.
pub(crate) fn lambda_order(code: CodeRef<'_>, lambdas: &Arena<LambdaData>) -> Vec<LambdaId> {
    fn walk(code: CodeRef<'_>, lambdas: &Arena<LambdaData>, order: &mut Vec<LambdaId>) {
        for child in lambda_children(code) {
            if order.contains(&child) {
                continue;
            }

            order.push(child);
            walk(lambdas[child].code(), lambdas, order);
        }
    }

    let mut order = Vec::new();

    walk(code, lambdas, &mut order);

    order
}

/// The lambdas a piece of code creates itself, in the order it creates them.
pub(crate) fn lambda_children(code: CodeRef<'_>) -> Vec<LambdaId> {
    let mut children = Vec::new();

    for (_, block) in code.blocks.iter() {
        for stmt in &block.stmts {
            let StmtKind::Assign {
                rvalue: Rvalue::Closure { lambda, .. },
                ..
            } = &stmt.kind
            else {
                continue;
            };

            if !children.contains(lambda) {
                children.push(*lambda);
            }
        }
    }

    children
}

/// What debug information `assemble_module` is asked for ([ADR-0025][adr-0025]).
///
/// A module carries one format and not two: a source map, which is what a browser reads
/// ([`DebugInfo::SourceMap`]), or the tables of DWARF, which is what `lldb` and `gdb` read
/// ([`DebugInfo::DwarfLines`], [`DebugInfo::DwarfFull`]), or none at all
/// ([`DebugInfo::None`]). The option is configuration, which is an input, so the debug
/// information of a module is equal for equal input ([ADR-0008][adr-0008]); the `name` section
/// is written at every option.
///
/// [adr-0008]: ../../docs/adr/0008-compiler-driver.md
/// [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugInfo {
    /// No debug information: the `name` section alone, which every module carries.
    None,
    /// The source map of a browser: the file, the line, and the column of every instruction,
    /// and the text of every file, carried by the module itself.
    #[default]
    SourceMap,
    /// DWARF line tables, a compile unit, and a subprogram per function.
    DwarfLines,
    /// DWARF line tables, functions, parameters, and locals.
    ///
    /// The DIEs of parameters and locals are a later milestone ([ADR-0025][adr-0025]); until
    /// they are emitted, this option carries what [`DebugInfo::DwarfLines`] carries.
    ///
    /// [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
    DwarfFull,
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
/// This is the whole of the back end for one module: one function per entity body and one per
/// lambda the bodies wrote, in the order the layout numbers them, and [`assemble_module`] over
/// the artifacts. It is what the driver's link stage and a host that shows one module both
/// read.
///
/// `lirs` are the lowered bodies of the module's functions, in declaration order, as the
/// driver pulled them ([ADR-0022][adr-0022]); `sources` are the files the bodies were read
/// from, which the debug information points at ([ADR-0025][adr-0025]).
///
/// # Panics
///
/// Panics when the number of bodies is not the number of functions of the module, which is a
/// mistake of the caller and not of the program.
///
/// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
/// [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
pub fn compile_module(
    module: &ModuleMir,
    lirs: &[Arc<LoweredFunction>],
    debug: DebugInfo,
    sources: &Sources,
) -> (WasmModule, Vec<CodegenDiag>) {
    assert_eq!(
        lirs.len(),
        module.functions.len(),
        "the back end is handed one LIR per function of the module",
    );

    let layout = layout(module);
    let builtins = layout.builtins().clone();
    let by_entity: BTreeMap<EntityLoc<FunctionLoc>, &ModuleFunction> = module
        .functions
        .iter()
        .filter_map(|function| {
            match &function.owner.item {
                BodyLoc::Function(loc) => {
                    Some((
                        EntityLoc {
                            module: function.owner.module,
                            item: loc.clone(),
                        },
                        function,
                    ))
                },
                BodyLoc::Const(_) => None,
            }
        })
        .collect();
    let mut functions = Vec::with_capacity(layout.len().saturating_sub(layout.imported()));
    let mut diagnostics = Vec::new();

    for lir in lirs {
        emit_tree(
            lir,
            &by_entity,
            &layout,
            &builtins,
            &mut functions,
            &mut diagnostics,
        );
    }

    let (wasm, reports) = assemble_module(module, &functions, debug, sources);

    diagnostics.extend(reports);

    (wasm, diagnostics)
}

/// Emits one function of a lowered tree and then the lambdas it wrote.
fn emit_tree(
    function: &LoweredFunction,
    by_entity: &BTreeMap<EntityLoc<FunctionLoc>, &ModuleFunction>,
    layout: &ModuleLayout,
    builtins: &Builtins,
    functions: &mut Vec<EmittedFunction>,
    diagnostics: &mut Vec<CodegenDiag>,
) {
    let (owner, name, signature, param_names, exported, lambda) = match &function.key {
        FunctionKey::Entity(entity) => {
            let declared = by_entity
                .get(entity)
                .expect("every entity of the module to be emitted");

            (
                declared.owner.clone(),
                declared.name.clone(),
                declared.signature.clone(),
                declared.param_names.clone(),
                declared.exported,
                None,
            )
        },
        FunctionKey::Lambda {
            owner: writer,
            lambda,
        } => {
            let plan = layout
                .lambda(writer, *lambda)
                .expect("every lambda of a body to be numbered");

            (
                writer.clone(),
                lambda_name(writer, *lambda),
                plan.signature.clone(),
                Vec::new(),
                false,
                Some(plan),
            )
        },
    };
    let type_index = match lambda {
        Some(plan) => plan.types.fn_type,
        None => {
            layout
                .plain_type(&signature.shape(builtins))
                .expect("every entity signature to have a plain function type")
        },
    };
    let arity = signature.params.len() as u32;
    let ctx = FunctionCtx {
        owner: &owner,
        name: &name,
        signature: &signature,
        param_names: &param_names,
        layout,
        lambda,
    };
    let (artifact, reports) = emit_function(&function.body, &ctx);

    diagnostics.extend(reports);
    functions.push(EmittedFunction {
        key: function.key.clone(),
        name,
        exported,
        arity,
        param_names,
        type_index,
        artifact,
    });

    for child in &function.lambdas {
        emit_tree(child, by_entity, layout, builtins, functions, diagnostics);
    }
}

/// The name of a lifted function: its owner and its place in the body, because a lambda has no
/// name of its own and a stack trace still needs one ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
pub(crate) fn lambda_name(owner: &BodyEntityLoc, lambda: LambdaId) -> String {
    let owner = owner
        .item
        .name()
        .map_or_else(|| format!("{:?}", owner.item), ToString::to_string);

    format!("{owner}::<mlkc@lambda-{}>", lambda.index())
}

/// Assembles the WASM module of `module` from the artifacts of its functions.
///
/// `functions` are the artifacts of every function the module declares --- the entity bodies
/// and the lambdas they wrote --- in the order the layout numbers them; `debug` is the format
/// of debug information the module carries ([`DebugInfo`]).
///
/// # Panics
///
/// Panics when the number of artifacts is not the number of functions of the module, which is
/// a mistake of the caller and not of the program: the driver emits one artifact per body it
/// lowered and nothing else.
pub fn assemble_module(
    module: &ModuleMir,
    functions: &[EmittedFunction],
    debug: DebugInfo,
    sources: &Sources,
) -> (WasmModule, Vec<CodegenDiag>) {
    let layout = layout(module);

    assert_eq!(
        functions.len(),
        layout.len() - layout.imported(),
        "the assembler is handed one artifact per function of the module",
    );

    // Every function of the module has the WASM type its shape gives it: an immediate is a
    // `(ref i31)` and every other type is a word, an `eqref`. One type per shape is all the
    // section needs, and the shapes are interned in the order the module meets them, so an
    // assignment of indices follows from the module alone ([ADR-0021][adr-0021]).
    //
    // The closures of the module add two kinds of types ([ADR-0026][adr-0026]): the shape group
    // of every function shape a closure is made of --- `(rec (type $fn ...) (type $closure
    // (sub (struct (field (ref $fn))))))` --- and the environment of every capture shape a
    // lambda creates, a final subtype of its shape's closure whose fields are the code and the
    // captures. A shape's group and an environment's group each hold exactly their own types:
    // the grouping is part of a type's identity, and an environment inside the shape's group
    // would make the shape differ between two modules that captured differently.
    //
    // [adr-0021]: ../../docs/adr/0021-translation-units.md
    // [adr-0026]: ../../docs/adr/0026-closure-representation.md
    let mut types = TypeSection::new();

    for shape in layout.plain_shapes() {
        types
            .ty()
            .function(shape.params.iter().map(|abi| abi.val_type()), [shape
                .ret
                .val_type()]);
    }

    for (at, shape) in layout.shape_groups().iter().enumerate() {
        let fn_type = layout.shape_base() + 2 * at as u32;

        // `$fn` takes the closure, and `$closure` holds the code ([ADR-0026]): the two
        // references are the group's two types, each to the other.
        //
        // [adr-0026]: ../../docs/adr/0026-closure-representation.md
        let closure_ref = reference(fn_type + 1);
        let function = SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Func(FuncType::new(
                    std::iter::once(closure_ref)
                        .chain(shape.params.iter().map(|abi| abi.val_type())),
                    [shape.ret.val_type()],
                )),
                shared: false,
                descriptor: None,
                describes: None,
            },
        };
        let closure = SubType {
            is_final: false,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Struct(StructType {
                    fields: vec![FieldType {
                        element_type: StorageType::Val(reference(fn_type)),
                        mutable: false,
                    }]
                    .into(),
                }),
                shared: false,
                descriptor: None,
                describes: None,
            },
        };

        types.ty().rec(vec![function, closure]);
    }

    for (shape, captures) in layout.env_groups() {
        let closure_types = layout
            .closure_types(shape)
            .expect("every environment shape to have a closure");
        let mut fields = Vec::with_capacity(1 + captures.len());

        fields.push(FieldType {
            element_type: StorageType::Val(reference(closure_types.fn_type)),
            mutable: false,
        });

        for capture in captures {
            fields.push(FieldType {
                element_type: StorageType::Val(capture.val_type()),
                mutable: false,
            });
        }

        types.ty().subtype(&SubType {
            is_final: true,
            supertype_idx: Some(closure_types.closure),
            composite_type: CompositeType {
                inner: CompositeInnerType::Struct(StructType {
                    fields: fields.into(),
                }),
                shared: false,
                descriptor: None,
                describes: None,
            },
        });
    }

    let mut import_section = ImportSection::new();
    let mut import_table = Vec::with_capacity(module.imports.len());

    for import in &module.imports {
        let shape = import.signature.shape(&module.builtins);
        let type_index = layout
            .plain_type(&shape)
            .expect("an import's shape to be in the type plan");

        import_section.import(
            &import.module,
            &import.name,
            EntityType::Function(type_index),
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

    for function in functions {
        function_section.function(function.type_index);
    }

    let mut exports = ExportSection::new();
    let mut export_table = Vec::new();

    for (index, function) in functions.iter().enumerate() {
        if function.exported {
            let index = offset + index as u32;

            exports.export(&function.name, ExportKind::Func, index);
            export_table.push(ExportDecl {
                module: module.name.clone(),
                name: function.name.clone(),
                arity: function.arity,
            });
        }
    }

    // A function a `ref.func` names must be declared, and a lifted lambda is only ever named
    // that way: the module declares every one of them in a declarative element segment.
    let declared: Vec<u32> = functions
        .iter()
        .enumerate()
        .filter(|(_, function)| matches!(function.key, FunctionKey::Lambda { .. }))
        .map(|(index, _)| offset + index as u32)
        .collect();

    let mut code = CodeSection::new();

    for function in functions {
        // `CodeSection::raw` writes the size prefix of the entry itself; the artifact holds the
        // body without it, because the layout is what the debug tables measure.
        code.raw(&function.artifact.body);
    }

    let code_layout = dwarf::Layout::of(functions);
    let mut wasm = wasm_encoder::Module::new();

    wasm.section(&types);
    wasm.section(&import_section);
    wasm.section(&function_section);
    wasm.section(&exports);

    if !declared.is_empty() {
        let mut elements = ElementSection::new();

        elements.declared(Elements::Functions(Cow::Borrowed(&declared)));
        wasm.section(&elements);
    }

    // A column of the source map is a byte offset in the module ([ADR-0025][adr-0025]), so the
    // offset of the contents of the code section --- after the `id` of the section and its
    // size --- is measured where the module stands when the code section is written. The layout
    // says how long the contents are, and the size is the length of them as an unsigned LEB128.
    //
    // [adr-0025]: ../../docs/adr/0025-debug-information-formats.md
    let code_payload =
        (wasm.as_slice().len() + 1 + dwarf::leb_len(code_layout.payload()) as usize) as u32;

    wasm.section(&code);

    let mut names = NameSection::new();

    names.module(&module.name);

    let mut function_names = NameMap::new();

    for (index, import) in module.imports.iter().enumerate() {
        function_names.append(index as u32, &import.name);
    }

    for (index, function) in functions.iter().enumerate() {
        function_names.append(offset + index as u32, &function.name);
    }

    names.functions(&function_names);

    let mut local_names = IndirectNameMap::new();

    for (index, function) in functions.iter().enumerate() {
        let mut names = NameMap::new();

        // The ABI parameters are locals of every function, and so are the locals the values
        // live in; a name is written for each of them where the source has one. An imported
        // function has no body and no locals of its own.
        for (parameter, name) in function.param_names.iter().enumerate() {
            if let Some(name) = name {
                names.append(parameter as u32, name.as_str());
            }
        }

        for value in &function.artifact.debug {
            let Some(name) = &value.name else {
                continue;
            };

            // A value that is a parameter lives in the local of that parameter, and the loop
            // above named it from the function; the artifact's name for it is the same one.
            if value.local < function.param_names.len() as u32 {
                continue;
            }

            names.append(value.local, name.as_str());
        }

        local_names.append(offset + index as u32, &names);
    }

    names.locals(&local_names);

    wasm.section(&names);

    match debug {
        DebugInfo::None => {},
        DebugInfo::SourceMap => {
            // The map carries the text of every file itself, so a browser needs no host to
            // serve it; the DWARF of a module would be preferred to it by an engine, and the
            // extension that reads DWARF has no sources to show ([`DebugInfo`]).
            if let Some((name, data)) =
                sourcemap::section(functions, sources, &code_layout, code_payload)
            {
                wasm.section(&wasm_encoder::CustomSection {
                    name: name.into(),
                    data: data.into(),
                });
            }
        },
        DebugInfo::DwarfLines | DebugInfo::DwarfFull => {
            for (name, data) in dwarf::sections(module, functions, sources, &code_layout) {
                wasm.section(&wasm_encoder::CustomSection {
                    name: name.into(),
                    data: data.into(),
                });
            }
        },
    }

    (
        WasmModule {
            bytes: wasm.finish(),
            imports: import_table,
            exports: export_table,
        },
        Vec::new(),
    )
}

/// A non-nullable reference to a concrete type of the module.
fn reference(ty: u32) -> ValType {
    ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(ty),
    })
}
