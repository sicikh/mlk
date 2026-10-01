//! The MIR of a body: the CFG form the checked body is lowered into, and its SSA form.

use std::sync::Arc;

use mlkc_hir_def::BodyEntityLoc;
use mlkc_mir::Body as MirBody;
use mlkc_mir_build::{construct_ssa, lower_body as lower_mir};
use mlkc_typeck::CheckDeps;

use super::{Driver, MirSlot, Pass, SsaSlot};

impl Driver {
    /// The MIR of `owner` in the CFG form: the checked body lowered, before SSA ([ADR-0019]).
    ///
    /// `None` when the body is not one the front end read clean: the file has no parse, the
    /// parser reported a mistake about it, the lowering of the HIR reported a mistake about the
    /// body, or the check of it did. A body whose meaning is a mistake has no MIR, so no back end
    /// meets an expression whose meaning is one.
    ///
    /// [ADR-0019]: ../../docs/adr/0019-mir.md
    pub fn mir(&mut self, owner: &BodyEntityLoc) -> Option<Arc<MirBody>> {
        self.checked_mir(owner)
    }

    /// The MIR of `owner` in the SSA form: the CFG form with block parameters ([ADR-0019]).
    ///
    /// The pass reads the CFG form and nothing else, so a second pull is the value the first one
    /// returned.
    ///
    /// [ADR-0019]: ../../docs/adr/0019-mir.md
    pub fn mir_ssa(&mut self, owner: &BodyEntityLoc) -> Option<Arc<MirBody>> {
        let cfg = self.mir(owner)?;
        let held = self.ssas.get(owner);

        if let Some(slot) = held
            && Arc::ptr_eq(&slot.cfg, &cfg)
        {
            self.stats.consulted(Pass::Ssa, true, true);

            return Some(slot.value.clone());
        }

        self.stats.consulted(Pass::Ssa, held.is_some(), false);

        let value = self.guarded(
            |driver| format!("building the SSA form of {}", driver.body_context(owner)),
            || construct_ssa(&cfg),
        )?;
        let value = Arc::new(value);

        // An SSA form equal to the one the driver holds is the value it holds ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match self.ssas.get(owner) {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::Ssa);

                slot.value.clone()
            },
            _ => value,
        };

        self.ssas.insert(owner.clone(), SsaSlot {
            cfg,
            value: value.clone(),
        });

        Some(value)
    }

    /// The MIR of one body in the CFG form, computed against the inputs of its check.
    fn checked_mir(&mut self, owner: &BodyEntityLoc) -> Option<Arc<MirBody>> {
        let module = owner.module();

        // A file the parser reported a mistake about is not compiled: a body of it may hold an
        // expression that is not there, and MIR has no meaning for one.
        if self.parse(module.0)?.has_errors() {
            return None;
        }

        let inputs = self.check_inputs(module)?;
        let checked = self.checked_with(owner, &inputs)?;

        // A body whose check reported a mistake is not lowered ([ADR-0019]): there is no meaning
        // to lower, and codegen never meets an expression whose meaning is a mistake.
        //
        // [ADR-0019]: ../../docs/adr/0019-mir.md
        if !checked.diagnostics.is_empty() {
            return None;
        }

        // What the driver holds for this body, when its key still says it was built from what
        // this pull read: the value is cloned out of the slot so that the borrow of the table
        // ends before the pass runs.
        let held = self.mirs.get(owner);
        let held = held
            .filter(|slot| {
                Arc::ptr_eq(&slot.lowered, &inputs.lowered)
                    && Arc::ptr_eq(&slot.resolution, &inputs.resolution)
                    && slot.closure.reads_the_same_as(&inputs.closure)
                    && slot.builtins == inputs.builtins
                    && slot.checked.reads_the_same_as(&checked)
            })
            .map(|slot| slot.value.clone());

        if let Some(value) = held {
            self.stats.consulted(Pass::Mir, true, true);

            return Some(value);
        }

        self.stats
            .consulted(Pass::Mir, self.mirs.contains_key(owner), false);

        let body = inputs
            .lowered
            .bodies()
            .iter()
            .find(|body| body.owner() == owner)?;

        // A body the HIR lowering reported a mistake about is not compiled: what it holds is not
        // a body the language means.
        if !body.body().diagnostics.is_empty() {
            return None;
        }

        // The surfaces the check read are not handed over: the lowering reads the types of the
        // check's value, and no surface of another module.
        let deps = CheckDeps::new(inputs.builtins.clone())
            .with_graph(Arc::clone(&self.projects))
            .with_closure(inputs.closure.clone());

        let value = self.guarded(
            |driver| format!("lowering {}", driver.body_context(owner)),
            || {
                lower_mir(
                    owner.clone(),
                    inputs.lowered.item_tree(),
                    &body.body().body,
                    &body.body().source_map,
                    &checked.value,
                    &inputs.resolution,
                    &deps,
                )
            },
        )?;
        let value = Arc::new(value);

        // MIR that ends up equal to the one the driver holds is the one it holds ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match self.mirs.get(owner) {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::Mir);

                slot.value.clone()
            },
            _ => value,
        };

        self.mirs.insert(owner.clone(), MirSlot {
            lowered: Arc::clone(&inputs.lowered),
            resolution: Arc::clone(&inputs.resolution),
            closure: inputs.closure.clone(),
            builtins: inputs.builtins.clone(),
            checked,
            value: value.clone(),
        });

        Some(value)
    }
}
