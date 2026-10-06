//! What a walk over the paths of a module answers.
//!
//! The walk is the machinery every stage that reads a path shares, and what is tested here is
//! what it answers for the anchors the lowering leaves: a name of the module, an entry of the
//! import table, a binding of a body, and a path rooted at a project.

use std::sync::Arc;

use mlkc_hir_def::{
    Attributes, ClassData, ClassLoc, Closure, EntityData, EntityLoc, ItemLoc, ItemLocLike,
    ItemSyntaxLoc, ItemTreeBuilder, ModuleId, ModuleScope, Name, Namespace, Pat, PathAnchor,
    PathData, PathRoot, PlainPath, PlainPathId, Prelude, ProjectGraph, Resolution, ResolveError,
    Target, UseLoc, Visibility, Walk, path::PathSegmentData,
};
use mlkc_lower::{lower_body, lower_module};
use mlkc_parser::parse;
use mlkc_syntax::ModuleRoot;
use mlkc_vfs::{FileId, RelPathBuf};

/// The module the tests of the walk are written in.
const MAIN: ModuleId = ModuleId(FileId::from_raw(0));

/// A module of another project, where the classes of the tests are declared.
const STD: ModuleId = ModuleId(FileId::from_raw(1));

/// The surface of the module of the tests: one import, and a body that binds a name.
const SOURCE: &str = "use project::std::Int\n\nfun main(x: Int): Int = x\n";

/// The module of the tests, lowered.
fn lowered() -> mlkc_lower::LoweredModule {
    let parsed = parse(SOURCE);
    let root = parsed.tree::<ModuleRoot>();
    let relative = RelPathBuf::try_from("main.mlk").expect("a relative path");

    lower_module(MAIN, &root, &Prelude::none(), &[], relative.as_path())
}

/// The class `Int` of the standard library of the tests.
fn class() -> EntityLoc<ClassLoc> {
    let mut builder = ItemTreeBuilder::new(
        STD,
        PlainPathId::new(PlainPath::from_root(PathRoot::Project, [Name::new("std")])),
    );
    builder.declare(
        Some(Name::new("Int")),
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

/// The import of the module of the tests.
fn import() -> UseLoc {
    lowered()
        .item_tree
        .entities()
        .find_map(|(item, _)| {
            match item {
                ItemLoc::Use(import) if item.name() == Some(&Name::new("Int")) => Some(import),
                _ => None,
            }
        })
        .expect("the import to be declared")
}

/// The resolution of the module of the tests: the imports a test hands over.
fn resolution(imports: &[(UseLoc, Target)]) -> Resolution {
    Resolution::new(
        Arc::new(ModuleScope::default()),
        imports.iter().cloned().collect(),
    )
}

/// A path of one name, anchored the way a test anchors it.
fn path(name: &str, anchor: PathAnchor) -> PathData {
    PathData::ident(Name::new(name), anchor)
}

#[test]
fn a_name_the_lowering_anchored_is_answered_as_it_is() {
    let class = class();
    let path = path("Int", PathAnchor::Item(EntityLoc::from(class.clone())));
    let graph = ProjectGraph::default();
    let closure = Closure::default();
    let mut walk = Walk::of(&graph, &closure);

    let (entity, errors) = walk.entity_of(MAIN, &resolution(&[]), &path, Namespace::Ty);

    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(entity, Some(EntityLoc::from(class)));
}

#[test]
fn a_name_written_after_an_entity_is_not_a_member_yet() {
    let class = class();
    let mut path = path("Int", PathAnchor::Item(EntityLoc::from(class)));
    path.segments.push(PathSegmentData {
        name: Name::new("field"),
        args: Vec::new(),
    });
    let graph = ProjectGraph::default();
    let closure = Closure::default();
    let mut walk = Walk::of(&graph, &closure);

    let (entity, errors) = walk.entity_of(MAIN, &resolution(&[]), &path, Namespace::Ty);

    assert_eq!(entity, None);
    assert_eq!(errors, [ResolveError::NestedName {
        name: Name::new("field"),
    }],);
}

#[test]
fn an_import_is_read_as_what_the_resolution_resolved() {
    let class = class();
    let import = import();
    let imports = [(
        import.clone(),
        Target::entity(Some(EntityLoc::from(class.clone())), None),
    )];
    let path = path("Int", PathAnchor::Use(import));
    let graph = ProjectGraph::default();
    let closure = Closure::default();
    let mut walk = Walk::of(&graph, &closure);

    let (entity, errors) = walk.entity_of(MAIN, &resolution(&imports), &path, Namespace::Ty);

    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(entity, Some(EntityLoc::from(class)));

    // The same path read where a value belongs: a class is not a value.
    let (entity, errors) = walk.entity_of(MAIN, &resolution(&imports), &path, Namespace::Value);

    assert_eq!(entity, None);
    assert_eq!(errors, [ResolveError::NotAValue {
        name: Name::new("Int"),
    }],);
}

#[test]
fn a_binding_is_answered_by_the_stage_that_owns_the_body() {
    let lowered = lowered();
    let decl = lowered.bodies.first().expect("a body to be declared");
    let body = lower_body(&lowered.item_tree, &decl.decl).expect("a body to be lowered");
    let pat = body.body.params()[0];

    assert!(matches!(body.body[pat], Pat::Bind(_)));

    let path = path("x", PathAnchor::Binding(pat));
    let graph = ProjectGraph::default();
    let closure = Closure::default();
    let mut walk = Walk::of(&graph, &closure);

    let (entity, errors) = walk.entity_of(MAIN, &resolution(&[]), &path, Namespace::Value);

    assert_eq!(entity, None);
    assert!(errors.is_empty());
}

#[test]
fn a_path_of_a_module_of_no_project_is_unresolved() {
    let path = PathData {
        root: PathRoot::Project,
        root_args: Vec::new(),
        segments: vec![PathSegmentData {
            name: Name::new("core"),
            args: Vec::new(),
        }],
        anchor: PathAnchor::Project(None),
    };
    let graph = ProjectGraph::default();
    let closure = Closure::default();
    let mut walk = Walk::of(&graph, &closure);

    let (entity, errors) = walk.entity_of(MAIN, &resolution(&[]), &path, Namespace::Ty);

    assert_eq!(entity, None);
    assert_eq!(errors, [ResolveError::NoProject]);
}
