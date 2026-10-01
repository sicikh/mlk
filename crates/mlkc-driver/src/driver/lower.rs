//! The HIR of a file: the surface of the module, its bodies, and where they are written.

use std::sync::Arc;

use mlkc_diagnostics::Diagnostic;
use mlkc_hir_def::{
    BodyEntityLoc, ItemLoc, ItemTree, ModuleId, Prelude, ProjectId, dump::TypePlace as DeclaredType,
};
use mlkc_lower::{LoweredBody, LoweringDiag, lower_body, lower_module, syntax_at};
use mlkc_rowan::AstNode;
use mlkc_syntax::{AnyParameter, FunDecl, ModuleRoot, TextRange};
use mlkc_vfs::{FileId, RelPath};
use rustc_hash::FxHashMap;

use super::{Driver, Pass};

/// The HIR of one file, and where what it holds is written.
///
/// The HIR is the surface of a module, the bodies of the entities that own one, and the names
/// of the module; what it does not hold is a place in the file, and a host that marks a buffer
/// with a range needs one. The driver holds the HIR and the syntax it was lowered from, so it
/// is the one that says where a node of the HIR is written.
pub struct Lowered {
    /// The surface of the module: its entities, their names, and their data.
    item_tree: ItemTree,
    /// Where each entity of the surface is written, in the bytes of the file.
    items: FxHashMap<ItemLoc, TextRange>,
    /// Where the types the declarations of the module write are written, by the entity that
    /// writes them.
    types: FxHashMap<ItemLoc, TypePlaces>,
    /// The bodies of the module, in the order it declares them.
    bodies: Vec<ModuleBody>,
    /// What lowering reported, as a host renders it.
    diagnostics: Arc<[Diagnostic]>,
}

impl Lowered {
    /// Runs the passes that follow the parse.
    ///
    /// The HIR of a module is lowered in two steps, and this is both of them: the surface of
    /// the module, and then each body, from the declaration it is written in.
    ///
    /// `relative` is where the file of the module stands, which is what the module is called
    /// when it declares no path of its own: see [`Driver::module_path`].
    fn of(
        module: ModuleId,
        root: &ModuleRoot,
        prelude: &Prelude,
        projects: &[ProjectId],
        relative: &RelPath,
    ) -> Self {
        let lowered = lower_module(module, root, prelude, projects, relative);

        // An entity the module did not write --- a prelude import --- is written nowhere, and
        // a host is given no range for it.
        let items = lowered
            .item_tree
            .entities()
            .filter_map(|(loc, id)| {
                let entity = lowered.item_tree.entity(id);

                Some((loc, syntax_at(root, entity.syntax()?)?.text_trimmed_range()))
            })
            .collect();

        let types = lowered
            .item_tree
            .entities()
            .filter_map(|(loc, id)| {
                let entity = lowered.item_tree.entity(id);
                let declaration = FunDecl::cast(syntax_at(root, entity.syntax()?)?)?;

                Some((loc, TypePlaces::of(&declaration)))
            })
            .collect();

        let bodies: Vec<ModuleBody> = lowered
            .bodies
            .iter()
            .filter_map(|decl| {
                let body = lower_body(&lowered.item_tree, &decl.decl)?;

                Some(ModuleBody {
                    owner: decl.owner.clone(),
                    body,
                })
            })
            .collect();

        // What the module says of its surface is reported before what its bodies say,
        // which is the order the module is read in.
        let diagnostics = lowered
            .diagnostics
            .iter()
            .chain(bodies.iter().flat_map(|it| it.body.diagnostics.iter()))
            .map(LoweringDiag::to_diagnostic)
            .collect::<Vec<_>>();

        Self {
            item_tree: lowered.item_tree,
            items,
            types,
            bodies,
            diagnostics: Arc::from(diagnostics),
        }
    }

    /// The surface of the module: its entities, their names, and their data.
    pub fn item_tree(&self) -> &ItemTree {
        &self.item_tree
    }

    /// Where the entity this name denotes is written, if the driver found where it is.
    ///
    /// A name is what crosses a revision, and a range is where it was written in this one:
    /// the two are the driver's to join, since the HIR holds the name and the syntax holds
    /// the place.
    pub fn item_range(&self, item: &ItemLoc) -> Option<TextRange> {
        self.items.get(item).copied()
    }

    /// Where the type a declaration writes at this place is written, if it writes one there.
    ///
    /// A type is a value of the HIR rather than a node of it, and the declaration a host reads
    /// it in is what says where it is written: the type of the parameter at an index, or the
    /// type the declaration writes for its result.
    pub fn type_range(&self, item: &ItemLoc, place: DeclaredType) -> Option<TextRange> {
        let places = self.types.get(item)?;

        match place {
            DeclaredType::Parameter(index) => places.params.get(index).copied().flatten(),
            DeclaredType::Result => places.result,
        }
    }

    /// The bodies of the module, in the order it declares them.
    pub fn bodies(&self) -> &[ModuleBody] {
        &self.bodies
    }

    /// What lowering reported, as a host renders it.
    pub fn diagnostics(&self) -> &Arc<[Diagnostic]> {
        &self.diagnostics
    }
}

/// One body of a module, and the entity that owns it.
pub struct ModuleBody {
    /// The entity that owns the body.
    owner: BodyEntityLoc,
    /// The body itself, with where its nodes are written.
    body: LoweredBody,
}

impl ModuleBody {
    /// The entity that owns the body.
    pub fn owner(&self) -> &BodyEntityLoc {
        &self.owner
    }

    /// The body: its expressions, its patterns, its paths, and where they are written.
    pub fn body(&self) -> &LoweredBody {
        &self.body
    }
}

/// Where the types of one declaration are written.
///
/// A signature is a value of the HIR, and where it was written is the declaration it was read
/// from: the type of a parameter is the annotation the parameter carries, and the result is
/// the type the declaration writes for it. A place the declaration writes no type at has none.
#[derive(Default)]
struct TypePlaces {
    /// The type of each parameter, by the index the declaration writes it at.
    params: Vec<Option<TextRange>>,
    /// The type the declaration writes for its result.
    result: Option<TextRange>,
}

impl TypePlaces {
    /// The places one declaration writes the types of its signature at.
    ///
    /// The i-th parameter of a signature is the i-th parameter of the declaration: a signature
    /// holds a parameter for every parameter the declaration wrote, the ones that broke
    /// included, which is what the lowering keeps the arity of a declaration for.
    fn of(declaration: &FunDecl) -> Self {
        let mut places = Self::default();

        if let Ok(parameters) = declaration.parameters() {
            places.params = parameters
                .items()
                .syntax()
                .children()
                .map(|node| {
                    let Some(AnyParameter::Parameter(parameter)) = AnyParameter::cast(node) else {
                        return None;
                    };

                    let ty = parameter.type_annotation()?.ty().ok()?;

                    Some(ty.syntax().text_trimmed_range())
                })
                .collect();
        }

        places.result = declaration
            .return_type_annotation()
            .and_then(|annotation| annotation.return_type().ok())
            .map(|ty| ty.syntax().text_trimmed_range());

        places
    }
}

impl Driver {
    /// The HIR of `file`, computed when the slot is missing or stale.
    ///
    /// `None` means there is nothing to lower: the file has no text, it was never parsed,
    /// the parse did not find a module in it, or the place of the file names no module.
    pub fn lower(&mut self, file: FileId) -> Option<Arc<Lowered>> {
        let Some(parse) = self.parse(file) else {
            self.stats
                .dropped(Pass::Lower, self.lowered.remove(&file).is_some() as usize);

            return None;
        };

        // Where the module stands, which is what it is called when it declares no path of its
        // own. It is read before the slot is taken: the lowering holds the slot of the HIR
        // while it runs.
        let Some(relative) = self.module_path(ModuleId(file)) else {
            // The place names no file, and a place that names no file names no module: there
            // is no path to call one by, and nothing to lower it to.
            self.stats
                .dropped(Pass::Lower, self.lowered.remove(&file).is_some() as usize);

            return None;
        };

        let version = self.file_version(file);
        let module = ModuleId(file);

        // The projects the module may name, which the lowering reads the name a type is rooted
        // at against ([ADR-0016]).
        //
        // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
        let projects: Vec<ProjectId> = self.named_projects(module);

        Self::text_derived(
            &mut self.stats,
            Pass::Lower,
            &mut self.lowered,
            file,
            version,
            || {
                let root = parse.module_root()?;

                // What the module is read with is the prelude of its project, which is the
                // prelude of the language for a module no project claims.
                let prelude = self.projects.prelude_of(module);

                Some(Arc::new(Lowered::of(
                    module,
                    &root,
                    prelude,
                    &projects,
                    relative.as_path(),
                )))
            },
        )
    }
}
