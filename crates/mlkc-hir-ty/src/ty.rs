//! A type, resolved.

use std::fmt;

use mlkc_hir_def::{ClassLoc, EntityLoc, ItemLocLike, TypeVarId};

/// The smallest `Int`: the payload of `i31ref` is 31 bits ([ADR-0018][adr-0018]).
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
pub const INT_MIN: i64 = -(1 << 30);

/// The largest `Int`: the payload of `i31ref` is 31 bits ([ADR-0018][adr-0018]).
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
pub const INT_MAX: i64 = (1 << 30) - 1;

/// A type, resolved: no path is left unresolved, and no inference variable survives.
///
/// A `Ty` owns everything it names, so it is a value of its own: it clones, compares, and
/// serializes without a store, and a reader in another module interprets it without retaining
/// the module that wrote it ([ADR-0010], [ADR-0017]).
///
/// [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    /// The type of something that could not be typed: broken syntax, a name that denotes
    /// nothing, a mistake already reported.
    ///
    /// It absorbs what it meets, so one mistake is one report.
    Error,

    /// A class, applied to its arguments: `Int`, `Map[Int, Str]`.
    Class {
        /// The class, named the way the project names it.
        class: EntityLoc<ClassLoc>,
        /// The arguments, in the order they are applied.
        args: Vec<Ty>,
    },

    /// A function: what it takes, and what it gives back.
    Fn {
        /// The parameters, in the order the function takes them.
        params: Vec<Ty>,
        /// The result.
        ret: Box<Ty>,
    },

    /// A parameter of the entity the type belongs to: the `a` of `forall a. a -> a`.
    ///
    /// It is the only variable a resolved type may hold: an inference variable is resolved or
    /// generalized before anything is stored ([ADR-0017]).
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    Param(TypeVarId),
}

impl Ty {
    /// The type of a class that takes no arguments.
    pub fn class(class: EntityLoc<ClassLoc>) -> Self {
        Self::Class {
            class,
            args: Vec::new(),
        }
    }

    /// The type of a function of `params` that gives back `ret`.
    pub fn function(params: Vec<Self>, ret: Self) -> Self {
        Self::Fn {
            params,
            ret: Box::new(ret),
        }
    }

    /// Whether the type is the type of a mistake.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// The parameters and the result of the type, if it is a function.
    pub fn as_function(&self) -> Option<(&[Self], &Self)> {
        match self {
            Self::Fn { params, ret } => Some((params, ret)),
            _ => None,
        }
    }
}

impl fmt::Display for Ty {
    /// The type as a reader of a message reads it: `Int`, `(Int, Bool) -> Int`,
    /// `{error}` for a mistake.
    ///
    /// A class reads as its name. The name is the one it was declared under, and not where it
    /// was declared: a message says the type, and a reader who wants to know which module it
    /// came from asks for the [`EntityLoc`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => f.write_str("{error}"),
            Self::Class { class, args } => {
                let name = class
                    .item
                    .name()
                    .map_or_else(|| format!("{:?}", class.item), ToString::to_string);
                f.write_str(&name)?;

                if !args.is_empty() {
                    f.write_str("[")?;
                    for (index, arg) in args.iter().enumerate() {
                        if index > 0 {
                            f.write_str(", ")?;
                        }
                        write!(f, "{arg}")?;
                    }
                    f.write_str("]")?;
                }

                Ok(())
            },
            Self::Fn { params, ret } => {
                f.write_str("(")?;
                for (index, param) in params.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{param}")?;
                }
                f.write_str(") -> ")?;

                // A result that is itself a function reads with its own parentheses, so that
                // `(Int) -> (Int) -> Int` does not read as two ways to say one thing.
                match ret.as_ref() {
                    Self::Fn { .. } => write!(f, "({ret})"),
                    ret => write!(f, "{ret}"),
                }
            },
            Self::Param(var) => write!(f, "'{}", var.index),
        }
    }
}

#[cfg(test)]
mod tests {
    use mlkc_hir_def::{
        Attributes, ClassData, ClassLoc, EntityData, EntityLoc, ItemSyntaxLoc, ItemTreeBuilder,
        ModuleId, Name, PathRoot, PlainPath, PlainPathId, TypeVarId, Visibility,
    };
    use mlkc_vfs::FileId;

    use super::Ty;

    fn class(name: &str) -> EntityLoc<ClassLoc> {
        let mut builder = ItemTreeBuilder::new(
            ModuleId(FileId::from_raw(0)),
            PlainPathId::new(PlainPath::from_root(PathRoot::Project, [Name::new("main")])),
        );
        builder.declare(
            Some(Name::new(name)),
            EntityData::Class(ClassData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
            }),
            ItemSyntaxLoc::root().child(0),
        );

        let tree = builder.finish();
        let (item, _) = tree.entities().next().expect("the class to be declared");

        EntityLoc {
            module: tree.module(),
            item: ClassLoc::try_from(item).expect("a class"),
        }
    }

    #[test]
    fn a_type_reads_as_the_names_it_is_made_of() {
        let int = Ty::class(class("Int"));
        let bool_ = Ty::class(class("Bool"));

        assert_eq!(int.to_string(), "Int");
        assert_eq!(Ty::Error.to_string(), "{error}");
        assert_eq!(
            Ty::function(vec![int.clone(), bool_.clone()], int.clone()).to_string(),
            "(Int, Bool) -> Int",
        );
        assert_eq!(
            Ty::function(vec![], Ty::function(vec![int.clone()], bool_.clone())).to_string(),
            "() -> ((Int) -> Bool)",
        );
        assert_eq!(
            Ty::Class {
                class: class("Map"),
                args: vec![int.clone(), bool_.clone()],
            }
            .to_string(),
            "Map[Int, Bool]",
        );
    }

    #[test]
    fn a_parameter_reads_as_a_letter_and_its_index() {
        let var = TypeVarId {
            owner: EntityLoc {
                module: ModuleId(FileId::from_raw(0)),
                item: mlkc_hir_def::ItemLoc::Class(class("Int").item),
            },
            index: 0,
        };

        assert_eq!(Ty::Param(var).to_string(), "'0");
    }

    #[test]
    fn a_function_reads_its_parts_back() {
        let ty = Ty::function(vec![Ty::class(class("Int"))], Ty::class(class("Bool")));
        let (params, ret) = ty.as_function().expect("a function");

        assert_eq!(params, [Ty::class(class("Int"))]);
        assert_eq!(ret, &Ty::class(class("Bool")));
        assert!(Ty::Error.as_function().is_none());
    }
}
