//! What an edit does to the HIR of a module.
//!
//! The driver keeps the values it lowered and compares them with the ones a new revision
//! produces, so that it re-uses what an edit did not change ([ADR-0008]). That is worth
//! something only if an edit invalidates as little as it can: what is tested here is that
//! lowering holds up its end of it --- an edit inside one body leaves the item tree of the
//! module equal and every other body equal, and an entity added between two others leaves
//! their data equal.
//!
//! [ADR-0008]: ../../docs/adr/0008-compiler-driver.md

use mlkc_hir_def::{Body, BodyLoc, EntityLoc, ItemLocLike, ItemTree, LocalEntry, ModuleId, Name};
use mlkc_lower::{LoweredModule, lower_body, lower_module};
use mlkc_parser_core::AnyParse;
use mlkc_syntax::ModuleRoot;
use mlkc_vfs::FileId;

/// A function, another one, and nothing between them.
const TWO_FUNCTIONS: &str = "\
fun first(): Int = 1

fun second(): Int = 2
";

/// The same module with the body of one function edited.
const EDITED_BODY: &str = "\
fun first(): Int = 1 + 1

fun second(): Int = 2
";

/// The same module with an entity declared between the two.
const INSERTED_FUNCTION: &str = "\
fun first(): Int = 1

fun added(): Int = 0

fun second(): Int = 2
";

fn parse(source: &str) -> AnyParse {
    mlkc_parser::parse(source)
}

fn lower(source: &str) -> LoweredModule {
    let parsed = parse(source);
    let root = parsed.tree::<ModuleRoot>();

    lower_module(ModuleId(FileId::from_raw(0)), &root)
}

/// The name of a module-level entity the module declares, as a dependent records it.
fn entity_loc(tree: &ItemTree, name: &str) -> EntityLoc {
    let entry = tree
        .scope()
        .get(&Name::new(name))
        .unwrap_or_else(|| panic!("the module to declare `{name}`"));

    match entry {
        LocalEntry::Item(item) => item.clone(),
        LocalEntry::Use(item) => panic!("`{name}` is an import, not an entity: {item:?}"),
    }
}

/// The body of the function `name`, lowered from the declaration it is written in.
fn body_of(lowered: &LoweredModule, name: &str) -> Body {
    let decl = lowered
        .bodies
        .iter()
        .find(|decl| {
            match decl.owner.item() {
                BodyLoc::Function(loc) => loc.name().is_some_and(|loc| loc == &Name::new(name)),
                BodyLoc::Const(_) => false,
            }
        })
        .unwrap_or_else(|| panic!("the module to declare a body of `{name}`"));

    lower_body(&lowered.item_tree, &decl.decl).body
}

#[test]
fn a_body_edit_leaves_the_item_tree_equal() {
    let before = lower(TWO_FUNCTIONS);
    let after = lower(EDITED_BODY);

    // The surface of the module did not change, so nothing that read it has to be read again.
    assert_eq!(before.item_tree, after.item_tree);
}

#[test]
fn a_body_edit_leaves_the_other_bodies_equal() {
    let before = lower(TWO_FUNCTIONS);
    let after = lower(EDITED_BODY);

    // The body that was edited is a new value, and the one that was not is the value the
    // driver already holds.
    assert_ne!(body_of(&before, "first"), body_of(&after, "first"));
    assert_eq!(body_of(&before, "second"), body_of(&after, "second"));
}

#[test]
fn an_added_entity_leaves_the_data_of_its_neighbours_alone() {
    let before = lower(TWO_FUNCTIONS);
    let after = lower(INSERTED_FUNCTION);

    // What a dependent of an entity read is the data of it, and the entities around the one
    // that was added are the same ones: the first declaration of a name is still what that
    // name means. The positions the item tree records do change, since the module is longer,
    // and a position is not what a dependent keys on.
    for name in ["first", "second"] {
        let loc = entity_loc(&before.item_tree, name);
        let data = before
            .item_tree
            .entity_data(loc.item.clone())
            .unwrap_or_else(|| panic!("the data of `{name}` to be in the tree"));

        assert_eq!(
            after.item_tree.entity_data(loc.item),
            Some(data),
            "the data of `{name}` is not the one it was",
        );
    }
}

#[test]
fn an_added_entity_leaves_the_bodies_of_its_neighbours_alone() {
    let before = lower(TWO_FUNCTIONS);
    let after = lower(INSERTED_FUNCTION);

    assert_eq!(body_of(&before, "first"), body_of(&after, "first"));
    assert_eq!(body_of(&before, "second"), body_of(&after, "second"));
}
