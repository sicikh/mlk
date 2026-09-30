//! The type surface of a module: the types of its entities, as the modules that read it see them.

use indexmap::IndexMap;
use mlkc_hir_def::{EntityLoc, ItemLocLike};

use crate::ty::Ty;

/// The types of a module's entities.
///
/// It is resolved from the signatures the module writes and from nothing else: no body is read
/// to build one ([ADR-0017]). A reader reads a type by the [`EntityLoc`] its resolution gave it,
/// and the value stands on its own --- no arena of the writer is retained to interpret it
/// ([ADR-0010]).
///
/// [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
/// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleTypes {
    /// The type of every entity that has one, in the order the module declares them.
    types: IndexMap<EntityLoc, Ty>,
}

impl ModuleTypes {
    /// A surface of no types.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the type of an entity, replacing what it had.
    pub fn insert(&mut self, entity: EntityLoc, ty: Ty) -> Option<Ty> {
        self.types.insert(entity, ty)
    }

    /// The type of an entity, if the surface holds one.
    pub fn get(&self, entity: &EntityLoc) -> Option<&Ty> {
        self.types.get(entity)
    }

    /// The types of the surface, in the order the module declares them.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&EntityLoc, &Ty)> {
        self.types.iter()
    }

    /// How many entities of the module have a type.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Whether the module declares no entity with a type.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

impl std::fmt::Display for ModuleTypes {
    /// The surface as a reader of a dump reads it, one entity per line:
    /// `fun double: (Int) -> Int`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, (entity, ty)) in self.types.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }

            let name = entity
                .item
                .name()
                .map_or_else(|| format!("{:?}", entity.item), ToString::to_string);

            write!(f, "{} {name}: {ty}", entity.item.kind().keyword())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mlkc_hir_def::{
        Attributes, ClassData, ClassLoc, EntityData, EntityLoc, FunctionData, ItemLoc,
        ItemSyntaxLoc, ItemTree, ItemTreeBuilder, ModuleId, Name, PathRoot, PlainPath, PlainPathId,
        Signature, Visibility,
    };
    use mlkc_vfs::FileId;

    use super::*;

    /// A module that declares a class `Int` and a function `double`.
    fn module() -> ItemTree {
        let mut builder = ItemTreeBuilder::new(
            ModuleId(FileId::from_raw(0)),
            PlainPathId::new(PlainPath::from_root(PathRoot::Project, [Name::new("main")])),
        );
        builder.declare(
            Some(Name::new("Int")),
            EntityData::Class(ClassData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
            }),
            ItemSyntaxLoc::root().child(0),
        );
        builder.declare(
            Some(Name::new("double")),
            EntityData::Function(FunctionData {
                attributes: Attributes::default(),
                visibility: Visibility::Public,
                signature: Signature::default(),
            }),
            ItemSyntaxLoc::root().child(1),
        );

        builder.finish()
    }

    /// The class `Int` of the module, and the name of the function `double`.
    fn entities(tree: &ItemTree) -> (EntityLoc<ClassLoc>, EntityLoc) {
        let mut int = None;
        let mut double = None;

        for (item, _) in tree.entities() {
            let entity = EntityLoc {
                module: tree.module(),
                item: item.clone(),
            };

            match item {
                ItemLoc::Class(_) => {
                    int = Some(EntityLoc {
                        module: tree.module(),
                        item: ClassLoc::try_from(item).expect("a class"),
                    })
                },
                ItemLoc::Function(_) if double.is_none() => double = Some(entity),
                _ => {},
            }
        }

        (int.expect("a class"), double.expect("a function"))
    }

    #[test]
    fn a_surface_holds_the_types_of_a_module_by_name_and_reads_as_lines() {
        let tree = module();
        let (int, double) = entities(&tree);

        let mut types = ModuleTypes::new();
        assert!(types.is_empty());
        assert_eq!(types.get(&double), None);

        types.insert(
            double.clone(),
            Ty::function(vec![Ty::class(int.clone())], Ty::class(int.clone())),
        );

        assert_eq!(
            types.get(&double),
            Some(&Ty::function(
                vec![Ty::class(int.clone())],
                Ty::class(int.clone())
            ))
        );
        assert_eq!(types.len(), 1);
        assert_eq!(types.iter().count(), 1);
        assert_eq!(types.to_string(), "fun double: (Int) -> Int");
    }
}
