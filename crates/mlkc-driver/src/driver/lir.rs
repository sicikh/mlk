//! The LIR of a body: the target's instructions in SSA form ([ADR-0022]).
//!
//! The pass reads the SSA form of the HIR body --- every function it declares, flat --- and the
//! module it belongs to: what the calls call, and the shape they cross as, are the module's. Each
//! function is lowered against the layout of that module --- a pure function of it --- and the
//! slot is keyed by the two values the pass read, so the pull is memoized per HIR body like the
//! MIR and its SSA form.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use std::sync::Arc;

use mlkc_codegen_wasm::{FunctionCtx, LoweredFunction, LoweredFunctions, layout, lower_function};
use mlkc_hir_def::{BodyEntityLoc, ModuleId};

use super::{Driver, LirSlot, Pass, Unit};

impl Driver {
    /// The LIR of `owner`: every function the HIR body declares, lowered into the instructions of
    /// the target ([ADR-0022]).
    ///
    /// `None` when the body has no SSA form, when the module it belongs to is not one the front
    /// end read clean, or when the lowering bugged: what a host reads of the last is the report
    /// of [`Driver::ice`]. A HIR body that declares no function of the module --- the body of a
    /// constant --- has no LIR at all.
    ///
    /// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
    pub fn lir(&mut self, owner: &BodyEntityLoc) -> Option<Arc<LoweredFunctions>> {
        let module = self.mir_module(owner.module())?;
        let ssa = self.mir_ssa(owner)?;
        let unit = Unit::Body(owner.clone());

        // What the driver holds for this body, when its key still says it was lowered against
        // what this pull read: the value is cloned out of the slot so that the borrow of the
        // table ends before the pass runs.
        let held = self
            .lirs
            .get(owner)
            .filter(|slot| Arc::ptr_eq(&slot.ssa, &ssa) && Arc::ptr_eq(&slot.module, &module))
            .map(|slot| Arc::clone(&slot.value));

        if let Some(value) = held {
            self.stats.consulted(Pass::Lir, &unit, true, true);

            return Some(value);
        }

        self.stats
            .consulted(Pass::Lir, &unit, self.lirs.contains_key(owner), false);

        let layout = layout(&module);
        let mut functions = Vec::with_capacity(ssa.len());

        for body in &ssa.bodies {
            // A body that is not a function of the module is the body of a constant: it has no
            // ABI and nothing calls it, so there is no LIR to write.
            let Some(declared) = module
                .functions
                .iter()
                .find(|function| function.function == body.function)
            else {
                continue;
            };
            let ctx = FunctionCtx {
                function: &declared.function,
                name: &declared.name,
                signature: &declared.signature,
                param_names: &declared.param_names,
                layout: &layout,
                lambda: layout.closure(&declared.function),
            };

            let started = self.ticking();
            let value = self.guarded(
                |driver| {
                    format!(
                        "lowering the LIR of {} of {}",
                        declared.name,
                        driver.body_context(owner),
                    )
                },
                || lower_function(body, &ctx),
            );

            self.stats.ran(Pass::Lir, &unit, self.clock, started);

            functions.push(Arc::new(value?));
        }

        if functions.is_empty() {
            return None;
        }

        let value = Arc::new(LoweredFunctions { functions });

        // A body equal to the one the driver holds is the value it holds ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match self.lirs.get(owner) {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::Lir, &unit);

                Arc::clone(&slot.value)
            },
            _ => value,
        };

        self.lirs.insert(owner.clone(), LirSlot {
            ssa,
            module,
            value: Arc::clone(&value),
        });

        Some(value)
    }

    /// The LIR of every function of `module`, in the order the module numbers them ([ADR-0022]).
    ///
    /// The bodies of the module are pulled HIR body by HIR body, because a HIR body is the unit
    /// of incrementality ([ADR-0003][adr-0003]), and read flat, because the module is.
    ///
    /// [adr-0003]: ../../docs/adr/0003-id-based-ir.md
    /// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
    pub fn module_lir(&mut self, module: ModuleId) -> Option<Vec<Arc<LoweredFunction>>> {
        let mir = self.mir_module(module)?;
        let lowered = self.lower(module.0)?;
        let mut functions = Vec::with_capacity(mir.functions.len());

        for body in lowered.bodies() {
            if let Some(lowered) = self.lir(body.owner()) {
                functions.extend(lowered.functions.iter().cloned());
            }
        }

        (functions.len() == mir.functions.len()).then_some(functions)
    }
}
