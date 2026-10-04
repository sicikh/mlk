//! The LIR of a body: the target's instructions in SSA form ([ADR-0022]).
//!
//! The pass reads the SSA form of the body and the module it belongs to: what its calls call,
//! and the shape they cross as, are the module's. The body is lowered against the layout of
//! that module --- a pure function of it --- and the slot is keyed by the two values the pass
//! read, so the pull is memoized per body like the MIR and its SSA form.
//!
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use std::sync::Arc;

use mlkc_codegen_wasm::{FunctionCtx, LoweredFunction, layout, lower_function};
use mlkc_hir_def::BodyEntityLoc;

use super::{Driver, LirSlot, Pass, Unit};

impl Driver {
    /// The LIR of `owner`: the body lowered into the instructions of the target ([ADR-0022]).
    ///
    /// `None` when the body has no SSA form, when the module it belongs to is not one the front
    /// end read clean, or when the lowering bugged: what a host reads of the last is the report
    /// of [`Driver::ice`].
    ///
    /// [adr-0022]: ../../docs/adr/0022-wasm-lir.md
    pub fn lir(&mut self, owner: &BodyEntityLoc) -> Option<Arc<LoweredFunction>> {
        let module = self.mir_module(owner.module())?;
        let function = module
            .functions
            .iter()
            .find(|function| &function.owner == owner)?;
        let ssa = Arc::clone(&function.body);
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
        let ctx = FunctionCtx {
            owner,
            name: &function.name,
            signature: &function.signature,
            param_names: &function.param_names,
            layout: &layout,
            lambda: None,
        };

        let started = self.ticking();
        let value = self.guarded(
            |driver| format!("lowering the LIR of {}", driver.body_context(owner)),
            || lower_function(&ssa, &ctx),
        );

        self.stats.ran(Pass::Lir, &unit, self.clock, started);

        let value = Arc::new(value?);

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
}
