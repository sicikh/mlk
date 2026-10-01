//! The types of a module: the signatures its declarations write, and the check of each body.

use std::{collections::BTreeMap, sync::Arc};

use mlkc_diagnostics::Diagnostic;
use mlkc_hir_def::{
    Body, BodyEntityLoc, ClassLoc, EntityData, EntityLoc, ItemLoc, ModuleId, Name, PathAnchor,
    ProjectId,
};
use mlkc_hir_ty::{CheckedBody, ModuleTypes};
use mlkc_resolve::Closure;
use mlkc_span::Span;
use mlkc_syntax::TextRange;
use mlkc_typeck::{
    Builtins, CheckDeps, TypeDiag, TypeError, TypePlace, check_body, resolve_module_types,
};

use super::{
    CheckInputs, CheckSlot, Checked, Driver, Lowered, ModuleBody, Signatures, SignaturesSlot,
    TypeDiagnosticsSlot, entries_are_the_same,
};

/// Where the place a check reported is written, if the driver found it.
///
/// An expression and a pattern are nodes of one body, and where they are written is the source
/// map of that body; an entity and a type a declaration writes are part of the surface of the
/// module, and where they are written is what the HIR of the module holds. A node the lowering
/// made up has no range of its own, and the declaration it stands in is what is marked for it
/// instead.
fn type_place_range(
    lowered: &Lowered,
    body: Option<&ModuleBody>,
    place: &TypePlace,
) -> Option<TextRange> {
    let declaration = || {
        let item = body.map(|body| ItemLoc::from(body.owner().item.clone()))?;
        lowered.item_range(&item)
    };

    match place {
        TypePlace::Expr(expr) => {
            body.and_then(|body| body.body().source_map.expr(*expr))
                .or_else(declaration)
        },
        TypePlace::Pat(pat) => {
            body.and_then(|body| body.body().source_map.pat(*pat))
                .or_else(declaration)
        },
        TypePlace::Entity(item) => lowered.item_range(item),
        TypePlace::Declared { item, place } => {
            lowered
                .type_range(item, *place)
                .or_else(|| lowered.item_range(item))
        },
    }
}

impl Driver {
    /// The classes of the language, as the standard library declares them.
    ///
    /// `Int`, `Unit`, `String`, and `Bool` are ordinary classes ([ADR-0017]): the library
    /// declares them with `#[builtin]`, and the check reads them by name rather than looking a
    /// primitive type up. A driver whose host recorded no library holds no classes to give, and
    /// checks no types: there is nothing for a literal or an operator to be.
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    fn builtins(&mut self) -> Option<Builtins> {
        let project = ProjectId::new(mlkc_stdlib::PROJECT);
        self.projects.project(&project)?;

        let index = self.module_index(&project)?;
        let core = index.get(&[Name::new(mlkc_stdlib::CORE)])?;
        let lowered = self.lower(core.0)?;
        let tree = lowered.item_tree();

        let class = |name: &str| -> Option<EntityLoc<ClassLoc>> {
            let target = tree.scope().get(&Name::new(name))?.ty.as_ref()?;
            let PathAnchor::Item(entity) = target.anchor() else {
                return None;
            };
            let EntityData::Class(data) = tree.entity_data(entity.item.clone())? else {
                return None;
            };

            // The language's class is the one the library declares as a builtin; a class that
            // happens to carry the name of one is a class like any other.
            if !data.attributes.builtin {
                return None;
            }

            Some(EntityLoc {
                module: entity.module,
                item: ClassLoc::try_from(entity.item).ok()?,
            })
        };

        Some(Builtins::new(
            class("Int")?,
            class("Unit")?,
            class("String")?,
            class("Bool")?,
        ))
    }

    /// The types of `module`'s entities, resolved from the signatures it writes ([ADR-0017]).
    ///
    /// The surface is a function of the module's own text and of the names it read, and never of
    /// a body: an edit inside a body leaves it where it was, and only the check of that body is
    /// read again.
    ///
    /// `None` when there is nothing to resolve the types against: the module has no HIR, or the
    /// driver holds no standard library, whose `#[builtin]` classes the check is given
    /// ([`Driver::use_std`]).
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn module_types(&mut self, module: ModuleId) -> Option<Arc<ModuleTypes>> {
        self.signatures(module).map(|signatures| signatures.value)
    }

    /// The type surface of `module`, and what resolving its written types reported.
    ///
    /// The key of the slot is what the pass read: the module's item tree, its resolution, and
    /// the closure its written types walk. A recomputation that ends up equal to what the driver
    /// holds is the value it holds, so a reader that came to the same surface came to nothing new
    /// ([ADR-0008]).
    ///
    /// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    fn signatures(&mut self, module: ModuleId) -> Option<Signatures> {
        let lowered = self.lower(module.0)?;
        let resolution = self.resolution(module)?;
        let builtins = self.builtins()?;
        let graph = Arc::clone(&self.projects);
        let indexes = self.indexes_of(module);

        let closure = Closure::of(
            module,
            lowered.item_tree(),
            &graph,
            &indexes,
            &mut |module| self.interface(module),
        );

        let held = self.signatures.get(&module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
            && Arc::ptr_eq(&slot.resolution, &resolution)
            && slot.closure.reads_the_same_as(&closure)
        {
            return Some(slot.signatures.clone());
        }

        let deps = CheckDeps::new(builtins)
            .with_graph(graph)
            .with_closure(closure.clone());
        let (value, diagnostics) = resolve_module_types(lowered.item_tree(), &resolution, &deps);
        let signatures = Signatures {
            value: Arc::new(value),
            diagnostics: Arc::from(diagnostics),
        };

        // A surface equal to the one the driver holds is the value it holds: a reader that came
        // to the same types came to nothing new ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let signatures = match held {
            Some(slot)
                if *slot.signatures.value == *signatures.value
                    && *slot.signatures.diagnostics == *signatures.diagnostics =>
            {
                slot.signatures.clone()
            },
            _ => signatures,
        };

        self.signatures.insert(module, SignaturesSlot {
            lowered,
            resolution,
            closure,
            signatures: signatures.clone(),
        });

        Some(signatures)
    }

    /// The check of one body: the types of its nodes, checked against the signatures its module
    /// wrote ([ADR-0017]).
    ///
    /// `None` when the body is not a body of a module the driver holds, or when there is nothing
    /// to check against: see [`Driver::module_types`].
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn check(&mut self, owner: &BodyEntityLoc) -> Option<Arc<CheckedBody>> {
        self.checked(owner).map(|checked| checked.value)
    }

    /// The check of one body, and what checking it reported.
    ///
    /// The key of the slot is what the check read: the HIR of the module --- the body among it
    /// --- the resolution, the wider closure of the check (the paths of every body, not only of
    /// the surface), the type surfaces of the modules the paths reach, and the classes of the
    /// language. A body edit changes the HIR, and the recomputation of a body that did not change
    /// ends up equal to what the driver holds, so the value a reader sees stays where it was.
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    fn checked(&mut self, owner: &BodyEntityLoc) -> Option<Checked> {
        let inputs = self.check_inputs(owner.module())?;

        self.checked_with(owner, &inputs)
    }

    /// What the check of a body reads of the units around it ([`CheckInputs`]).
    ///
    /// `None` when there is nothing to check against: the module has no HIR, it has no resolution,
    /// or the driver holds no standard library, whose `#[builtin]` classes the check is given
    /// ([`Driver::use_std`]).
    pub(super) fn check_inputs(&mut self, module: ModuleId) -> Option<CheckInputs> {
        let lowered = self.lower(module.0)?;
        let resolution = self.resolution(module)?;
        let builtins = self.builtins()?;
        let graph = Arc::clone(&self.projects);
        let indexes = self.indexes_of(module);

        // The closure of the check is wider than the closure of a resolution: a path of a body
        // may name a module that no signature names ([ADR-0017]).
        let bodies: Vec<&Body> = lowered
            .bodies()
            .iter()
            .map(|body| &body.body().body)
            .collect();
        let closure = Closure::of_check(
            module,
            lowered.item_tree(),
            bodies,
            &graph,
            &indexes,
            &mut |module| self.interface(module),
        );

        // Every module the check may read a type from: its own, and the ones its paths reach.
        let mut modules: Vec<ModuleId> = closure.interfaces().keys().copied().collect();
        modules.push(module);
        modules.sort_unstable();
        modules.dedup();

        let mut types: BTreeMap<ModuleId, Arc<ModuleTypes>> = BTreeMap::new();

        for named in modules {
            if let Some(signatures) = self.signatures(named) {
                types.insert(named, signatures.value);
            }
        }

        Some(CheckInputs {
            lowered,
            resolution,
            closure,
            types,
            builtins,
        })
    }

    /// The check of a body against inputs already gathered ([`Driver::check_inputs`]).
    pub(super) fn checked_with(
        &mut self,
        owner: &BodyEntityLoc,
        inputs: &CheckInputs,
    ) -> Option<Checked> {
        let CheckInputs {
            lowered,
            resolution,
            closure,
            types,
            builtins,
        } = inputs;
        let held = self.checks.get(owner);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, lowered)
            && Arc::ptr_eq(&slot.resolution, resolution)
            && slot.closure.reads_the_same_as(closure)
            && entries_are_the_same(&slot.types, types)
            && slot.builtins == *builtins
        {
            return Some(slot.checked.clone());
        }

        let body = lowered.bodies().iter().find(|body| body.owner() == owner)?;
        let mut deps = CheckDeps::new(builtins.clone())
            .with_graph(Arc::clone(&self.projects))
            .with_closure(closure.clone());

        for (named, surface) in types {
            deps = deps.with_types(*named, Arc::clone(surface));
        }

        let (value, diagnostics) = check_body(
            owner.clone(),
            lowered.item_tree(),
            &body.body().body,
            resolution,
            &deps,
        );
        let checked = Checked {
            value: Arc::new(value),
            diagnostics: Arc::from(diagnostics),
        };

        // A check that ends up equal to the one the driver holds is the one it holds: a body edit
        // neither moves the types of the bodies that did not change nor the values that read
        // them ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let checked = match held {
            Some(slot)
                if *slot.checked.value == *checked.value
                    && *slot.checked.diagnostics == *checked.diagnostics =>
            {
                slot.checked.clone()
            },
            _ => checked,
        };

        self.checks.insert(owner.clone(), CheckSlot {
            lowered: Arc::clone(lowered),
            resolution: Arc::clone(resolution),
            closure: closure.clone(),
            types: types.clone(),
            builtins: builtins.clone(),
            checked: checked.clone(),
        });

        Some(checked)
    }

    /// The diagnostics of the checks of `module`'s bodies, in the shape a host renders.
    ///
    /// A check reports places in the HIR --- an expression, a pattern, a type a declaration
    /// writes --- and where a place is written is what the driver holds, so the rendering is the
    /// driver's. What a path of a body could not find is looked up in the module the path
    /// reached, exactly as the rendering of a resolution does ([`hidden_name`]), and what the
    /// look read is what the slot is keyed by.
    ///
    /// The reports of the module come first --- what resolving its signatures found, in the
    /// order it declares them --- and then the reports of its bodies, in the order it declares
    /// them.
    pub(super) fn type_diagnostics(&mut self, module: ModuleId) -> Option<Arc<[Diagnostic]>> {
        let lowered = self.lower(module.0)?;
        let signatures = self.signatures(module)?;

        // Every body of the module is checked, in the order the module declares them.
        let checks: Vec<Checked> = lowered
            .bodies()
            .iter()
            .map(|body| self.checked(body.owner()))
            .collect::<Option<Vec<_>>>()?;

        let held = self.type_diagnostics.get(&module);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.lowered, &lowered)
            && slot.signatures.reads_the_same_as(&signatures)
            && slot.checks.len() == checks.len()
            && slot
                .checks
                .iter()
                .zip(&checks)
                .all(|(held, current)| held.reads_the_same_as(current))
        {
            // What the rendering looked at is the HIR of the modules a name the walk could not
            // find may belong to: a look that names the same HIR is a look that read the same
            // thing, and the value that was rendered from it is the value the driver holds.
            let looked: Vec<(ModuleId, Arc<Lowered>)> = slot
                .looked
                .iter()
                .map(|(module, lowered)| (*module, lowered.clone()))
                .collect();

            if looked.iter().all(|(module, held)| {
                self.lower(module.0)
                    .is_some_and(|current| Arc::ptr_eq(held, &current))
            }) {
                return Some(self.type_diagnostics[&module].value.clone());
            }
        }

        let mut looked = BTreeMap::new();
        let mut rendered = Vec::new();

        // What resolving the signatures reported belongs to the module as a whole, and it comes
        // before what its bodies reported, which is the order the module is read in.
        for diagnostic in signatures.diagnostics.iter() {
            rendered.push(self.rendered_type(diagnostic, module, &lowered, None, &mut looked));
        }

        for (body, checked) in lowered.bodies().iter().zip(&checks) {
            for diagnostic in checked.diagnostics.iter() {
                rendered.push(self.rendered_type(
                    diagnostic,
                    module,
                    &lowered,
                    Some(body),
                    &mut looked,
                ));
            }
        }

        let value: Arc<[Diagnostic]> = Arc::from(rendered);

        // A rendering equal to the one the driver holds is the value it holds: an edit that moved
        // a body but not what a host reads does not move the value the buffer is marked by
        // ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match self.type_diagnostics.get(&module) {
            Some(slot) if *slot.value == *value => slot.value.clone(),
            _ => value,
        };

        self.type_diagnostics.insert(module, TypeDiagnosticsSlot {
            lowered,
            signatures,
            checks,
            looked,
            value: value.clone(),
        });

        Some(value)
    }

    /// The diagnostic a host renders of what a check found.
    ///
    /// A name a path of a body could not find may be a name the module at the end of the path
    /// holds and does not show, and telling that is a look at the module itself: what the look
    /// read is recorded, since it is what the rendering is keyed by ([`hidden_name`]).
    fn rendered_type(
        &mut self,
        diagnostic: &TypeDiag,
        file: ModuleId,
        lowered: &Lowered,
        body: Option<&ModuleBody>,
        looked: &mut BTreeMap<ModuleId, Arc<Lowered>>,
    ) -> Diagnostic {
        // A place the driver wrote no range for has nowhere to point at: what the check found is
        // still what a host is told about, and it is told without a place.
        let Some(range) = type_place_range(lowered, body, diagnostic.place()) else {
            return Diagnostic::from_kind(diagnostic.error(), diagnostic.error().message());
        };

        let span = Span::new(file.0, range);

        // The paths of a body are walked by the check and never by the resolution ([ADR-0016]),
        // so a name a body walk could not find is the check's to report --- and the look that
        // tells a name a module keeps to itself is the look the rendering of a resolution
        // takes.
        //
        // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
        let error = match diagnostic.error() {
            TypeError::Unresolved { error } => self.hidden(error, looked),
            _ => None,
        };

        match error {
            Some(hidden) => diagnostic.to_diagnostic_with(span, hidden.message()),
            None => diagnostic.to_diagnostic(span),
        }
    }
}
