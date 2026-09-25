//! The observable data of an entity: everything a dependent may read.
//!
//! The data of an entity is a value of its own.
//! It owns the paths and the type references it names, so it can be compared, retained,
//! and serialized without the item tree it was cut from,
//! and a dependent keys on the data it read instead of on the whole module surface ([ADR-0010]).
//!
//! [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md

use crate::{
    def_map::LocalScope,
    id::ItemKind,
    macros::{define_entity_data, for_each_item_kind},
    name::Name,
    path::PlainPathId,
    type_ref::TypeRef,
};

/// How far an entity is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Visibility {
    /// Visible to every module of the project.
    Public,
    /// Visible only inside the module that declares the entity.
    #[default]
    Private,
}

impl Visibility {
    /// Whether the entity is visible outside its module.
    pub fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }
}

/// The signature of a function: what a caller has to know about it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Signature {
    /// The parameters, in the order they are declared.
    pub params: Vec<ParamData>,
    /// The type of the result, which a public function declares and a private one may leave out.
    pub ret: Option<TypeRef>,
}

impl Signature {
    /// Resolves the anchors the paths of this signature left unresolved.
    ///
    /// Every type of a signature is read where a type belongs: the names it is made of are
    /// looked for in the type namespace of the module.
    pub(crate) fn resolve(&mut self, scope: &LocalScope) {
        for param in &mut self.params {
            if let Some(ty) = &mut param.ty {
                ty.resolve(scope);
            }
        }
        if let Some(ret) = &mut self.ret {
            ret.resolve(scope);
        }
    }
}

/// One parameter of a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamData {
    /// The name the parameter is declared under.
    pub name: Name,
    /// The type as written, if it was written.
    pub ty: Option<TypeRef>,
}

/// What a module-level function is to everything that is not its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionData {
    /// How far the function is visible.
    pub visibility: Visibility,
    /// The signature a caller reads.
    pub signature: Signature,
}

/// What a class is to everything that is not its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassData {
    /// How far the class is visible.
    pub visibility: Visibility,
}

/// What a module-level value is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueData {
    /// How far the value is visible.
    pub visibility: Visibility,
}

/// What a module-level constant is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstData {
    /// How far the constant is visible.
    pub visibility: Visibility,
    /// The type of the constant, as written.
    pub ty: Option<TypeRef>,
}

/// What an `impl` is: the class it implements and the type it implements it for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplData {
    /// The class, as written.
    pub class: Option<TypeRef>,
    /// The type, as written.
    pub ty: Option<TypeRef>,
}

/// What a `use` brings into the module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseData {
    /// The path as written.
    pub path: PlainPathId,
    /// The name the import is declared under, if the path is not the name.
    pub alias: Option<Name>,
    /// How far the imported name is visible.
    pub visibility: Visibility,
}

for_each_item_kind!(define_entity_data);

impl EntityData {
    /// The visibility of the entity, if its kind has one of its own.
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
            Self::Function(data) => Some(data.visibility),
            Self::Class(data) => Some(data.visibility),
            Self::Value(data) => Some(data.visibility),
            Self::Const(data) => Some(data.visibility),
            Self::Use(data) => Some(data.visibility),
            Self::Impl(_) => None,
        }
    }

    /// Resolves the anchors the paths of this data left unresolved.
    ///
    /// The paths a kind of entity holds are read where the language writes them: a class, a
    /// constant, and an `impl` name types, and a `use` names an import, which is a path of the
    /// import table rather than a path in a namespace of the module.
    pub(crate) fn resolve(&mut self, scope: &LocalScope) {
        match self {
            Self::Function(data) => data.signature.resolve(scope),
            Self::Const(data) => {
                if let Some(ty) = &mut data.ty {
                    ty.resolve(scope);
                }
            },
            Self::Impl(data) => {
                if let Some(class) = &mut data.class {
                    class.resolve(scope);
                }
                if let Some(ty) = &mut data.ty {
                    ty.resolve(scope);
                }
            },
            // A class and a value have no types of their own yet,
            // and an import names its target by a path, not by the HIR's own resolution.
            Self::Class(_) | Self::Value(_) | Self::Use(_) => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::*;
    use crate::{
        id::{ClassLoc, EntityLoc, ItemLoc, ItemLocData, ModuleId},
        path::{PathAnchor, PathData},
        type_ref::TypeRef,
    };

    fn module() -> ModuleId {
        ModuleId(FileId::from_raw(0))
    }

    fn class(name: &str) -> EntityLoc {
        EntityLoc {
            module: module(),
            item: ItemLoc::Class(ClassLoc(ItemLocData {
                name: Some(Name::new(name)),
                disambiguator: 0,
            })),
        }
    }

    #[test]
    fn a_signature_resolves_its_anchors() {
        let mut scope = LocalScope::default();
        scope.declare(
            Name::new("T"),
            crate::def_map::LocalTarget::Item(class("T")),
        );

        let mut data = FunctionData {
            visibility: Visibility::Public,
            signature: Signature {
                params: vec![ParamData {
                    name: Name::new("x"),
                    ty: Some(TypeRef::Path(PathData::ident(
                        Name::new("T"),
                        PathAnchor::Unresolved,
                    ))),
                }],
                ret: Some(TypeRef::Infer),
            },
        };
        data.signature.resolve(&scope);

        assert_eq!(
            data.signature.params[0].ty,
            Some(TypeRef::Path(PathData::ident(
                Name::new("T"),
                PathAnchor::Item(class("T")),
            ))),
        );
    }

    #[test]
    fn the_visibility_of_an_impl_is_not_a_thing() {
        let data = EntityData::Impl(ImplData {
            class: None,
            ty: None,
        });

        assert_eq!(data.visibility(), None);
        assert_eq!(data.kind().as_str(), "Impl");
    }

    #[test]
    fn a_default_visibility_is_private() {
        assert_eq!(Visibility::default(), Visibility::Private);
        assert!(!Visibility::Private.is_public());
        assert!(Visibility::Public.is_public());
    }
}
