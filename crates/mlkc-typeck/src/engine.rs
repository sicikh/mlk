//! The inference engine of one body check: variables, unification, levels, and generalization.
//!
//! Nothing here survives the check: a variable is either bound by unification, generalized into
//! a [`InferTy::Param`] of the entity, or turned into the error type when a stored type is built
//! ([ADR-0017]). Generalization is the level-based algorithm of Rémy, as the note [okmij]
//! explains: a variable carries the level of the `let` that created it, unification lowers the
//! level of a variable that meets an older one, and what is still owned by a `let` when the
//! `let` ends is generalized.
//!
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md
//! [okmij]: https://okmij.org/ftp/ML/generalization.html

use mlkc_hir_def::{ClassLoc, EntityLoc, TypeVarId};
use mlkc_hir_ty::Ty;

/// The id of an inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VarId(u32);

/// A type as the check sees it: a resolved type, or a variable it does not know yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InferTy {
    /// The type of a mistake, which absorbs what it meets.
    Error,
    /// A class, applied to its arguments.
    Class {
        /// The class.
        class: EntityLoc<ClassLoc>,
        /// The arguments.
        args: Vec<InferTy>,
    },
    /// A function.
    Fn {
        /// The parameters.
        params: Vec<InferTy>,
        /// The result.
        ret: Box<InferTy>,
    },
    /// A parameter of the entity being checked: a variable that never unifies with anything less
    /// general than itself.
    Param(TypeVarId),
    /// A variable the check does not know yet.
    Var(VarId),
}

impl InferTy {
    /// A resolved type, as the check sees it.
    pub(crate) fn of(ty: &Ty) -> Self {
        match ty {
            Ty::Error => Self::Error,
            Ty::Class { class, args } => {
                Self::Class {
                    class: class.clone(),
                    args: args.iter().map(Self::of).collect(),
                }
            },
            Ty::Fn { params, ret } => {
                Self::Fn {
                    params: params.iter().map(Self::of).collect(),
                    ret: Box::new(Self::of(ret)),
                }
            },
            Ty::Param(var) => Self::Param(var.clone()),
        }
    }

    /// Whether the type is the type of a mistake.
    pub(crate) fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }
}

/// What unification failed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UnifyError {
    /// Two types that have to be one are two.
    Mismatch {
        /// The type a place expects.
        expected: InferTy,
        /// The type of what is written there.
        found: InferTy,
    },
    /// A variable would contain itself.
    Recursive,
}

/// A generalized type: what a `let` binds, and what a call of a polymorphic entity uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Scheme {
    /// The type, with a [`InferTy::Param`] in place of every generalized variable.
    pub(crate) ty: InferTy,
    /// The parameters the type was generalized over, in the order they were named.
    pub(crate) params: Vec<TypeVarId>,
}

/// One inference variable.
#[derive(Debug, Clone)]
struct Var {
    /// The level of the `let` that created it.
    level: u32,
    /// What it stands for, once unification bound it.
    binding: Option<InferTy>,
}

/// The engine of one body check.
///
/// The types of the entity being checked are its parameters: [`Engine::owner`] names them.
pub(crate) struct Engine {
    /// The variables, by id.
    vars: Vec<Var>,
    /// The level of the innermost `let` whose right side is being inferred.
    level: u32,
    /// The entity whose check generalized a parameter.
    owner: EntityLoc,
    /// How many parameters have been named so far.
    next_param: u32,
}

impl Engine {
    /// An engine that checks the body of `owner`.
    pub(crate) fn new(owner: EntityLoc) -> Self {
        Self {
            vars: Vec::new(),
            level: 0,
            owner,
            next_param: 0,
        }
    }

    /// A fresh variable, owned by the innermost `let`.
    pub(crate) fn fresh_var(&mut self) -> InferTy {
        self.vars.push(Var {
            level: self.level,
            binding: None,
        });

        InferTy::Var(VarId(self.vars.len() as u32 - 1))
    }

    /// Enters the region of a `let` whose right side is about to be inferred.
    pub(crate) fn enter(&mut self) {
        self.level += 1;
    }

    /// Leaves the region of a `let`, which is what makes the variables of it generalizable.
    pub(crate) fn leave(&mut self) {
        self.level -= 1;
    }

    /// Unifies what a place expects with what is written there.
    pub(crate) fn unify(&mut self, expected: &InferTy, found: &InferTy) -> Result<(), UnifyError> {
        let expected = self.repr(expected);
        let found = self.repr(found);

        match (expected, found) {
            // A mistake reports once and does not cascade.
            (InferTy::Error, _) | (_, InferTy::Error) => Ok(()),
            (InferTy::Var(left), InferTy::Var(right)) if left == right => Ok(()),
            (InferTy::Var(var), ty) => self.bind(var, &ty),
            (ty, InferTy::Var(var)) => self.bind(var, &ty),
            (InferTy::Param(left), InferTy::Param(right)) if left == right => Ok(()),
            (
                InferTy::Class {
                    class: left,
                    args: left_args,
                },
                InferTy::Class {
                    class: right,
                    args: right_args,
                },
            ) if left == right && left_args.len() == right_args.len() => {
                for (left, right) in left_args.iter().zip(&right_args) {
                    self.unify(left, right)?;
                }

                Ok(())
            },
            (
                InferTy::Fn {
                    params: left,
                    ret: left_ret,
                },
                InferTy::Fn {
                    params: right,
                    ret: right_ret,
                },
            ) if left.len() == right.len() => {
                for (left, right) in left.iter().zip(&right) {
                    self.unify(left, right)?;
                }

                self.unify(&left_ret, &right_ret)
            },
            (expected, found) => Err(UnifyError::Mismatch { expected, found }),
        }
    }

    /// Binds a variable to a type, if the type can hold it.
    fn bind(&mut self, var: VarId, ty: &InferTy) -> Result<(), UnifyError> {
        let ty = self.repr(ty);

        // Two variables: the younger one is bound to the older one. The older one is what
        // another occurrence of either will find, so this is what lowers a level.
        if let InferTy::Var(other) = ty {
            if other == var {
                return Ok(());
            }

            let level = self.vars[var.0 as usize].level;
            let other_level = self.vars[other.0 as usize].level;

            return if other_level <= level {
                self.vars[var.0 as usize].binding = Some(InferTy::Var(other));
                Ok(())
            } else {
                self.vars[other.0 as usize].binding = Some(InferTy::Var(var));
                Ok(())
            };
        }

        let level = self.vars[var.0 as usize].level;
        self.occurs(var, &ty, level)?;
        self.vars[var.0 as usize].binding = Some(ty);

        Ok(())
    }

    /// Checks that `ty` does not contain `var`, lowering the level of the variables it does
    /// contain to `level`.
    fn occurs(&mut self, var: VarId, ty: &InferTy, level: u32) -> Result<(), UnifyError> {
        match ty {
            InferTy::Error | InferTy::Param(_) => Ok(()),
            InferTy::Var(other) if *other == var => Err(UnifyError::Recursive),
            InferTy::Var(other) => {
                if let Some(binding) = self.vars[other.0 as usize].binding.clone() {
                    return self.occurs(var, &binding, level);
                }

                let slot = &mut self.vars[other.0 as usize];
                if slot.level > level {
                    slot.level = level;
                }

                Ok(())
            },
            InferTy::Class { args, .. } => {
                for arg in args {
                    self.occurs(var, arg, level)?;
                }

                Ok(())
            },
            InferTy::Fn { params, ret } => {
                for param in params {
                    self.occurs(var, param, level)?;
                }

                self.occurs(var, ret, level)
            },
        }
    }

    /// Generalizes a type: every variable still owned by the innermost ended `let` becomes a
    /// parameter of the entity being checked.
    pub(crate) fn generalize(&mut self, ty: &InferTy) -> Scheme {
        let mut params = Vec::new();
        let ty = self.quantify(ty, &mut params);

        Scheme { ty, params }
    }

    /// What [`Engine::generalize`] walks with.
    fn quantify(&mut self, ty: &InferTy, params: &mut Vec<TypeVarId>) -> InferTy {
        match self.repr(ty) {
            InferTy::Var(var) => {
                if self.vars[var.0 as usize].level > self.level {
                    let param = TypeVarId {
                        owner: self.owner.clone(),
                        index: self.next_param,
                    };
                    self.next_param += 1;
                    params.push(param.clone());

                    // The variable is the parameter from here on. Binding it is what makes the
                    // occurrences a stored type still holds --- the type of the lambda a `let`
                    // generalized, above all --- read as that parameter, and what keeps a second
                    // occurrence of the variable from being named a second time.
                    self.vars[var.0 as usize].binding = Some(InferTy::Param(param.clone()));

                    InferTy::Param(param)
                } else {
                    // The variable belongs to a `let` that is still being inferred, or to the
                    // signature of the entity: it is not this `let`'s to quantify.
                    InferTy::Var(var)
                }
            },
            InferTy::Class { class, args } => {
                InferTy::Class {
                    class,
                    args: args.iter().map(|arg| self.quantify(arg, params)).collect(),
                }
            },
            InferTy::Fn { params: ps, ret } => {
                InferTy::Fn {
                    params: ps
                        .iter()
                        .map(|param| self.quantify(param, params))
                        .collect(),
                    ret: Box::new(self.quantify(&ret, params)),
                }
            },
            ty => ty,
        }
    }

    /// Replaces the listed parameters with fresh variables: one variable per parameter, and the
    /// same variable wherever the parameter occurs in the scheme.
    pub(crate) fn instantiate(&mut self, ty: &InferTy, params: &[TypeVarId]) -> InferTy {
        let fresh: Vec<InferTy> = params.iter().map(|_| self.fresh_var()).collect();

        self.substitute(ty, params, &fresh)
    }

    /// What [`Engine::instantiate`] walks with: the fresh variable of every parameter, in the
    /// order of the parameters.
    fn substitute(&self, ty: &InferTy, params: &[TypeVarId], fresh: &[InferTy]) -> InferTy {
        match self.repr(ty) {
            InferTy::Param(var) => {
                match params.iter().position(|param| *param == var) {
                    Some(index) => fresh[index].clone(),
                    None => InferTy::Param(var),
                }
            },
            InferTy::Class { class, args } => {
                InferTy::Class {
                    class,
                    args: args
                        .iter()
                        .map(|arg| self.substitute(arg, params, fresh))
                        .collect(),
                }
            },
            InferTy::Fn { params: ps, ret } => {
                InferTy::Fn {
                    params: ps
                        .iter()
                        .map(|param| self.substitute(param, params, fresh))
                        .collect(),
                    ret: Box::new(self.substitute(&ret, params, fresh)),
                }
            },
            ty => ty,
        }
    }

    /// Replaces every parameter of `owner` with a fresh variable: what a use of an entity does
    /// with the signature it read.
    pub(crate) fn instantiate_owned(&mut self, ty: &InferTy, owner: &EntityLoc) -> InferTy {
        let mut params = Vec::new();
        self.params_of(ty, owner, &mut params);
        self.instantiate(ty, &params)
    }

    /// Collects the parameters of `owner` a type holds.
    fn params_of(&self, ty: &InferTy, owner: &EntityLoc, params: &mut Vec<TypeVarId>) {
        match self.repr(ty) {
            InferTy::Param(var) => {
                if &var.owner == owner && !params.contains(&var) {
                    params.push(var);
                }
            },
            InferTy::Class { args, .. } => {
                for arg in &args {
                    self.params_of(arg, owner, params);
                }
            },
            InferTy::Fn { params: ps, ret } => {
                for param in &ps {
                    self.params_of(param, owner, params);
                }
                self.params_of(&ret, owner, params);
            },
            _ => {},
        }
    }

    /// The type to store: what the variables stand for, and the error type for one that was
    /// never resolved.
    pub(crate) fn zonk(&self, ty: &InferTy) -> Ty {
        match self.repr(ty) {
            InferTy::Error | InferTy::Var(_) => Ty::Error,
            InferTy::Class { class, args } => {
                Ty::Class {
                    class,
                    args: args.iter().map(|arg| self.zonk(arg)).collect(),
                }
            },
            InferTy::Fn { params, ret } => {
                Ty::Fn {
                    params: params.iter().map(|param| self.zonk(param)).collect(),
                    ret: Box::new(self.zonk(&ret)),
                }
            },
            InferTy::Param(var) => Ty::Param(var),
        }
    }

    /// What a variable stands for, with the links of bound variables followed.
    pub(crate) fn repr(&self, ty: &InferTy) -> InferTy {
        let mut ty = ty.clone();

        while let InferTy::Var(var) = ty {
            match self.vars[var.0 as usize].binding.clone() {
                Some(binding) => ty = binding,
                None => break,
            }
        }

        ty
    }
}

#[cfg(test)]
mod tests {
    use mlkc_hir_def::{
        Attributes, ClassData, ClassLoc, EntityData, EntityLoc, FunctionData, ItemLoc,
        ItemSyntaxLoc, ItemTreeBuilder, ModuleId, Name, PathRoot, PlainPath, PlainPathId,
        Signature, Visibility,
    };
    use mlkc_hir_ty::Ty;
    use mlkc_vfs::FileId;

    use super::*;

    /// A module with a function `f`, which a check generalizes over, and a class `Int`.
    fn entities() -> (EntityLoc, EntityLoc<ClassLoc>) {
        let mut builder = ItemTreeBuilder::new(
            ModuleId(FileId::from_raw(0)),
            PlainPathId::new(PlainPath::from_root(PathRoot::Project, [Name::new("main")])),
        );
        builder.declare(
            Some(Name::new("f")),
            EntityData::Function(FunctionData {
                attributes: Attributes::default(),
                visibility: Visibility::Private,
                signature: Signature::default(),
            }),
            ItemSyntaxLoc::root().child(0),
        );
        builder.declare(
            Some(Name::new("Int")),
            EntityData::Class(ClassData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
            }),
            ItemSyntaxLoc::root().child(1),
        );

        let tree = builder.finish();
        let mut owner = None;
        let mut int = None;

        for (item, _) in tree.entities() {
            match &item {
                ItemLoc::Function(_) if owner.is_none() => {
                    owner = Some(EntityLoc {
                        module: tree.module(),
                        item: item.clone(),
                    });
                },
                ItemLoc::Class(_) => {
                    int = Some(EntityLoc {
                        module: tree.module(),
                        item: ClassLoc::try_from(item.clone()).expect("a class"),
                    });
                },
                _ => {},
            }
        }

        (owner.expect("a function"), int.expect("a class"))
    }

    fn function(params: Vec<InferTy>) -> InferTy {
        InferTy::Fn {
            params,
            ret: Box::new(InferTy::Error),
        }
    }

    #[test]
    fn a_variable_unifies_with_a_type_and_is_read_back() {
        let (owner, int) = entities();
        let mut engine = Engine::new(owner);
        let var = engine.fresh_var();
        let int_ty = InferTy::Class {
            class: int.clone(),
            args: Vec::new(),
        };

        assert_eq!(engine.unify(&var, &int_ty), Ok(()));
        // Both directions find what the variable stands for.
        assert_eq!(engine.unify(&int_ty, &var), Ok(()));
        assert_eq!(engine.zonk(&var), Ty::class(int));
    }

    #[test]
    fn two_functions_of_different_shapes_do_not_unify() {
        let (owner, _) = entities();
        let mut engine = Engine::new(owner);
        let var = engine.fresh_var();

        assert_eq!(engine.unify(&var, &function(vec![InferTy::Error])), Ok(()));
        assert!(matches!(
            engine.unify(&var, &function(vec![InferTy::Error, InferTy::Error])),
            Err(UnifyError::Mismatch { .. })
        ));
    }

    #[test]
    fn a_variable_does_not_contain_itself() {
        let (owner, _) = entities();
        let mut engine = Engine::new(owner);
        let var = engine.fresh_var();
        let function = InferTy::Fn {
            params: vec![var.clone()],
            ret: Box::new(InferTy::Error),
        };

        assert_eq!(engine.unify(&var, &function), Err(UnifyError::Recursive));
    }

    #[test]
    fn what_a_let_created_is_generalized_and_instantiated_fresh() {
        let (owner, _) = entities();
        let mut engine = Engine::new(owner);

        engine.enter();
        let var = engine.fresh_var();
        engine.leave();

        let scheme = engine.generalize(&var);
        assert_eq!(scheme.params.len(), 1);
        assert_eq!(scheme.ty, InferTy::Param(scheme.params[0].clone()));

        let first = engine.instantiate(&scheme.ty, &scheme.params);
        let second = engine.instantiate(&scheme.ty, &scheme.params);
        assert!(matches!(&first, InferTy::Var(_)));
        assert_ne!(first, second);
    }

    #[test]
    fn a_parameter_is_one_fresh_variable_wherever_it_occurs() {
        let (owner, _) = entities();
        let mut engine = Engine::new(owner);

        engine.enter();
        let var = engine.fresh_var();
        engine.leave();

        // The scheme of `fn(x) -> x`: the parameter is read in the parameters and in the
        // result, and is one variable of the entity either way.
        let scheme = engine.generalize(&InferTy::Fn {
            params: vec![var.clone()],
            ret: Box::new(var),
        });
        assert_eq!(scheme.params.len(), 1);
        assert_eq!(scheme.ty, InferTy::Fn {
            params: vec![InferTy::Param(scheme.params[0].clone())],
            ret: Box::new(InferTy::Param(scheme.params[0].clone())),
        },);

        let InferTy::Fn { params, ret } = engine.instantiate(&scheme.ty, &scheme.params) else {
            panic!("a function");
        };

        assert!(matches!(params[0], InferTy::Var(_)));
        assert_eq!(params[0], *ret);
    }

    #[test]
    fn what_an_outer_let_created_is_not_generalized_by_an_inner_one() {
        let (owner, _) = entities();
        let mut engine = Engine::new(owner);

        let outer = engine.fresh_var();

        engine.enter();
        let inner = engine.fresh_var();
        engine.leave();

        let scheme = engine.generalize(&InferTy::Fn {
            params: vec![outer.clone(), inner],
            ret: Box::new(InferTy::Error),
        });

        assert_eq!(scheme.params.len(), 1);

        let InferTy::Fn { params, .. } = &scheme.ty else {
            panic!("a function");
        };
        assert_eq!(params[0], outer);
        assert!(matches!(params[1], InferTy::Param(_)));
    }
}
