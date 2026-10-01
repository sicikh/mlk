//! What the tests of the crate share.
//!
//! A body is the body of an entity, and an entity's name is minted by an item tree and never
//! built by hand ([ADR-0010][adr-0010]); so a test that builds a body of its own builds a
//! module of one function and takes the name from it.
//!
//! [adr-0010]: ../../docs/adr/0010-stable-entity-identity.md

use mlkc_hir_def::{
    Attributes, BodyEntityLoc, BodyLoc, EntityData, EntityLoc, FunctionData, ItemLoc,
    ItemSyntaxLoc, ItemTreeBuilder, ModuleId, Name, PathRoot, PlainPath, PlainPathId, Signature,
    Visibility,
};
use mlkc_vfs::FileId;

/// The owner of a body of a test: the first function of a module of one.
pub(crate) fn owner() -> BodyEntityLoc {
    let mut builder = ItemTreeBuilder::new(
        ModuleId(FileId::from_raw(0)),
        PlainPathId::new(PlainPath::from_root(PathRoot::Project, [Name::new("main")])),
    );
    builder.declare(
        Some(Name::new("main")),
        EntityData::Function(FunctionData {
            attributes: Attributes::default(),
            visibility: Visibility::Public,
            signature: Signature::default(),
        }),
        ItemSyntaxLoc::root().child(0),
    );

    let tree = builder.finish();
    let (item, _) = tree.entities().next().expect("the function to be declared");
    let ItemLoc::Function(function) = item else {
        panic!("the entity is a function");
    };

    EntityLoc {
        module: tree.module(),
        item: BodyLoc::Function(function),
    }
}
