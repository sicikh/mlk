//! What a module shows to the modules that read it.
//!
//! An interface is a function of the module's own text alone: the projection of an item tree
//! onto what another module may read. It is the value a resolution reads of another module,
//! and the key of everything that read it ([ADR-0008], [ADR-0016]).
//!
//! [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md

use indexmap::IndexMap;

use crate::{
    def_map::Namespace,
    id::{EntityLoc, ItemLocLike},
    item_data::{EntityData, Visibility},
    item_tree::ItemTree,
    name::Name,
    path::PlainPathId,
};

/// What a module shows to the modules that name it.
///
/// It holds the path the module is called by and the names it exports --- the ones it declares
/// publicly, and the ones a public `use` brings in --- and nothing behind them: an exported
/// name is either a name of this module or the path a re-export wrote, left unresolved.
/// A module is therefore never read while an interface is built, and an interface can be
/// compared, retained, and serialized on its own ([ADR-0008], [ADR-0010]).
///
/// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
/// [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    /// The path the module is called by in its project.
    path: PlainPathId,
    /// The names the module exports, in the order it declares them.
    exports: IndexMap<Name, Export>,
}

/// What one exported name of a module denotes to the modules that read it.
///
/// The namespaces a name is declared in are the namespaces of the kind of the entity:
/// a class is a type, and a function, a value, and a constant are values. What an import
/// brings in is different: which namespace it lands in is what the path it wrote resolves to,
/// and a module alone cannot say, so the path is held for the modules that read the interface.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Export {
    /// The entity the module declares under this name, where a type belongs.
    pub ty: Option<EntityLoc>,
    /// The entity the module declares under this name, where a value belongs.
    pub value: Option<EntityLoc>,
    /// The path a public `use` of the name wrote, as it wrote it: unresolved.
    pub reexport: Option<PlainPathId>,
}

impl Export {
    /// The entity the name denotes in `namespace`, if the module declares one there.
    pub fn entity(&self, namespace: Namespace) -> Option<&EntityLoc> {
        match namespace {
            Namespace::Ty => self.ty.as_ref(),
            Namespace::Value => self.value.as_ref(),
            // An entity of the module is never in the module namespace: only a name an import
            // brings in is, and what it is is what the import resolves to.
            Namespace::Module => None,
        }
    }

    /// Whether the name denotes nothing at all.
    pub fn is_empty(&self) -> bool {
        self.ty.is_none() && self.value.is_none() && self.reexport.is_none()
    }
}

impl Interface {
    /// The interface of a module: what its item tree shows.
    ///
    /// An entity the module does not export is not in it: a private declaration is a name of
    /// the module and not of the modules that read it, and an `impl` has no name of its own.
    /// The first declaration of a name wins, as it does in the scope of the module.
    pub fn of(tree: &ItemTree) -> Self {
        let mut exports: IndexMap<Name, Export> = IndexMap::new();

        for (item, id) in tree.entities() {
            let Some(name) = item.name() else {
                continue;
            };

            let data = tree.entity(id).data();

            if !data.visibility().is_some_and(Visibility::is_public) {
                continue;
            }

            let export = exports.entry(name.clone()).or_default();

            match data {
                // What an import brings in is the path it wrote: which namespace the name
                // lands in is what that path resolves to.
                EntityData::Use(data) => {
                    if export.reexport.is_none() {
                        export.reexport = Some(data.path.clone());
                    }
                },
                data => {
                    let entity = EntityLoc {
                        module: tree.module(),
                        item: item.clone(),
                    };

                    for &namespace in data.kind().namespaces() {
                        match namespace {
                            Namespace::Ty => export.ty.get_or_insert_with(|| entity.clone()),
                            Namespace::Value => export.value.get_or_insert_with(|| entity.clone()),
                            Namespace::Module => continue,
                        };
                    }
                },
            }
        }

        Self {
            path: tree.path(),
            exports,
        }
    }

    /// The path the module is called by in its project.
    pub fn path(&self) -> &PlainPathId {
        &self.path
    }

    /// What an exported name denotes, if the module exports one.
    pub fn export(&self, name: &Name) -> Option<&Export> {
        self.exports.get(name)
    }

    /// The names the module exports, in the order it declares them.
    pub fn exports(&self) -> impl Iterator<Item = (&Name, &Export)> {
        self.exports.iter()
    }

    /// How many names the module exports.
    pub fn len(&self) -> usize {
        self.exports.len()
    }

    /// Whether the module exports no name at all.
    pub fn is_empty(&self) -> bool {
        self.exports.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use mlkc_vfs::FileId;

    use super::*;
    use crate::{
        id::{ItemKind, ItemLoc, ModuleId},
        item_data::{Attributes, ClassData, FunctionData, Signature, UseData},
        item_tree::{ItemSyntaxLoc, ItemTreeBuilder},
        path::{PathRoot, PlainPath, PlainPathId},
    };

    fn module() -> ModuleId {
        ModuleId(FileId::from_raw(0))
    }

    fn path(name: &str) -> PlainPathId {
        PlainPathId::new(PlainPath::from_root(PathRoot::Project, [Name::new(name)]))
    }

    fn builder() -> ItemTreeBuilder {
        ItemTreeBuilder::new(module(), path("main"))
    }

    /// The data of a class, named for the reader: a name lives in the entity's loc, and the
    /// builder mints it from what `declare` is given.
    fn class(_name: &str, visibility: Visibility) -> EntityData {
        EntityData::Class(ClassData {
            attributes: Attributes::default(),
            visibility,
        })
    }

    fn import(name: &str, visibility: Visibility) -> EntityData {
        EntityData::Use(UseData {
            path: path(name),
            alias: None,
            visibility,
        })
    }

    #[test]
    fn an_interface_holds_the_public_names_of_a_module_and_the_path_it_is_called_by() {
        let mut builder = builder();
        builder.declare(
            Some(Name::new("Int")),
            class("Int", Visibility::Public),
            ItemSyntaxLoc::root().child(0),
        );
        builder.declare(
            Some(Name::new("helper")),
            class("helper", Visibility::Private),
            ItemSyntaxLoc::root().child(1),
        );
        builder.declare(
            Some(Name::new("Unit")),
            import("Unit", Visibility::Public),
            ItemSyntaxLoc::root().child(2),
        );

        let interface = Interface::of(&builder.finish());

        assert_eq!(interface.path().to_string(), "project::main");
        assert_eq!(interface.len(), 2);

        let int = interface
            .export(&Name::new("Int"))
            .expect("`Int` to be exported");
        assert_eq!(
            int.ty.as_ref().map(|loc| loc.item.clone()),
            Some(ItemLoc::new(ItemKind::Class, Some(Name::new("Int")), 0))
        );
        assert_eq!(int.reexport, None);

        // A private name is a name of the module, and not one the modules that read it see.
        assert_eq!(interface.export(&Name::new("helper")), None);

        // A public import is a re-export: the path is held as the module wrote it.
        let unit = interface
            .export(&Name::new("Unit"))
            .expect("`Unit` to be exported");
        assert_eq!(
            unit.reexport.as_ref().map(ToString::to_string),
            Some("project::Unit".to_owned())
        );
        assert!(unit.ty.is_none() && unit.value.is_none());
    }

    #[test]
    fn an_import_the_module_did_not_write_is_not_exported() {
        let mut builder = builder();
        builder.declare_prelude(&crate::prelude::PreludeImport::new(
            Name::new("Int"),
            path("Int"),
        ));

        let interface = Interface::of(&builder.finish());

        // The prelude import is private: it is a name of the module, and not one it shows.
        assert!(interface.is_empty());
    }

    #[test]
    fn the_first_declaration_of_a_name_wins_in_an_interface() {
        let mut builder = builder();
        builder.declare(
            Some(Name::new("Box")),
            class("Box", Visibility::Public),
            ItemSyntaxLoc::root().child(0),
        );
        builder.declare(
            Some(Name::new("Box")),
            EntityData::Function(FunctionData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
                signature: Signature::default(),
            }),
            ItemSyntaxLoc::root().child(1),
        );

        let interface = Interface::of(&builder.finish());

        // A class and a function of one name are in different namespaces, and both are shown.
        let export = interface
            .export(&Name::new("Box"))
            .expect("`Box` to be exported");
        assert_eq!(
            export.ty.as_ref().map(|loc| loc.item.kind()),
            Some(ItemKind::Class)
        );
        assert_eq!(
            export.value.as_ref().map(|loc| loc.item.kind()),
            Some(ItemKind::Function)
        );
    }
}
