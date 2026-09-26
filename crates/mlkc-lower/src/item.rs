//! The surface of one module: the items it declares, and the bodies they own.

use mlkc_hir_def::{
    Attributes, BodyEntityLoc, ClassData, ClassLoc, EntityData, EntityLoc, FunctionData,
    FunctionLoc, ItemLoc, ItemSyntaxLoc, ItemTree, ItemTreeBuilder, LocalTarget, ModuleId, Name,
    PlainPathId, UseData, UseLoc, Visibility,
};
use mlkc_rowan::AstNode;
use mlkc_syntax::{AttributeList, FunDecl, ModuleItem, ModuleRoot, SyntaxNode, TypeDecl, UseDecl};
use mlkc_vfs::FileId;

use crate::{
    BodyDecl, LoweredModule, LoweringDiag, LoweringError, decl, path,
    syntax::{self, item_position, syntax_at},
};

/// Lowers the items of `root` into the item tree of `module`.
pub(crate) fn lower(module: ModuleId, root: &ModuleRoot) -> LoweredModule {
    let file = module.0;
    let mut lowering = ItemLowering {
        root,
        file,
        builder: ItemTreeBuilder::new(module),
        diagnostics: Vec::new(),
        bodies: Vec::new(),
    };

    lowering.preamble();
    lowering.items();

    let item_tree = lowering.builder.finish();

    // A rule about the names of a module as a whole is checked once all of them are known:
    // whether an import repeats a declaration is not a question about the order the two were
    // written in, and the two of them are in the tree by now.
    let mut diagnostics = lowering.diagnostics;
    diagnostics.extend(imports_of_declared_names(&item_tree, root, file));

    // A reader reads a module from its top, and what lowering found is read the same way: the
    // rules are checked in the order the declarations are walked, and the mistakes are ordered
    // by where they are written rather than by when they were found. The order is therefore
    // the same however the checking of a rule is arranged, and a diff of a module reads in the
    // order of the module.
    diagnostics.sort_by_key(|diagnostic| diagnostic.span().range.start());

    LoweredModule {
        item_tree,
        diagnostics,
        bodies: lowering.bodies,
    }
}

/// The names that an import brings in and the module declares.
///
/// A name means one thing in a module, and a module that both imports a name and declares one
/// of it says two: what a use of the name means is the declaration, which is what the module
/// wrote itself, and the import of it is what a reader is told about --- so the diagnostic
/// points at the import, wherever in the module it is written.
fn imports_of_declared_names(
    tree: &ItemTree,
    root: &ModuleRoot,
    file: FileId,
) -> Vec<LoweringDiag> {
    let mut diagnostics = Vec::new();

    for (name, entry) in tree.scope().iter() {
        let Some(import) = &entry.import else {
            continue;
        };

        // An import is in the way of every namespace: which one it lands in is what the import
        // resolves to, so a declaration in any namespace is a declaration of its name.
        let Some((_, declared)) = entry.iter().next() else {
            continue;
        };

        // A namespace holds an entity of the module: an import is held apart from them.
        let LocalTarget::Item(declaration) = declared else {
            continue;
        };

        let Some(node) = declared_import(tree, root, import) else {
            continue;
        };

        let error = LoweringError::ImportOfDeclaredName {
            name: name.clone(),
            declaration: declaration.clone(),
        };

        diagnostics.push(LoweringDiag::new(error, syntax::span(file, &node)));
    }

    diagnostics
}

/// The syntax of an import, found by the position the item tree recorded for it.
fn declared_import(tree: &ItemTree, root: &ModuleRoot, import: &UseLoc) -> Option<SyntaxNode> {
    let position = tree.syntax_loc(ItemLoc::Use(import.clone()))?;

    syntax_at(root, position)
}

/// The lowering of the items of one module.
struct ItemLowering<'a> {
    root: &'a ModuleRoot,
    file: FileId,
    builder: ItemTreeBuilder,
    diagnostics: Vec<LoweringDiag>,
    bodies: Vec<BodyDecl>,
}

impl ItemLowering<'_> {
    /// Records the path the module declares itself as, which is what its preamble writes.
    ///
    /// A module that has no preamble declares no path: what it is called is what the project
    /// it belongs to says, and the path of the file is the canonical form of that.
    fn preamble(&mut self) {
        let Some(preamble) = self.root.preamble() else {
            return;
        };

        // A preamble whose path the parser could not read declares nothing:
        // a path of no segments names nothing, and the parse is what reported the mistake.
        let Ok(path) = preamble.name() else {
            return;
        };

        self.plain_path(&path);
        self.builder.set_path(PlainPathId::new(path::plain(&path)));
    }

    /// Declares every item of the module, in the order it is written.
    fn items(&mut self) {
        let list = self.root.items().syntax().clone();

        for node in list.children() {
            let position = item_position(&list, &node);

            // An item the parser could not read is what the parse reported, and the HIR holds
            // what the module does say: a broken item says nothing.
            match ModuleItem::cast(node) {
                Some(ModuleItem::FunDecl(decl)) => self.function(&decl, position),
                Some(ModuleItem::TypeDecl(decl)) => self.class(&decl, position),
                Some(ModuleItem::UseDecl(decl)) => self.import(&decl, position),
                Some(ModuleItem::BogusDecl(_)) | None => {},
            }
        }
    }

    /// Declares one function, and remembers the body it owns.
    fn function(&mut self, decl: &FunDecl, position: ItemSyntaxLoc) {
        let attributes = self.attributes(&decl.attributes());
        let visibility = decl::visibility(decl.visibility_token());
        let signature = decl::signature(decl, self.file, &mut self.diagnostics);

        let data = EntityData::Function(FunctionData {
            attributes,
            visibility,
            signature,
        });

        let loc = self.entity(
            decl.syntax(),
            position,
            Some(syntax::name(decl.name())),
            data,
        );
        let function = FunctionLoc::try_from(loc.item.clone()).expect("a function is a function");

        self.function_rules(decl, &function, attributes, visibility);

        // The declaration is an item of the module whether it has a body or not: what a
        // function without one is, is its signature, and what owns a body is the entity.
        if decl.body().is_some() {
            let owner = BodyEntityLoc::try_from(loc).expect("a function owns a body");

            self.bodies.push(BodyDecl {
                owner,
                decl: decl.clone(),
            });
        }
    }

    /// Reports what the language asks of the function itself: what its parameters say of it,
    /// what its attributes say of it, and what a caller of it needs.
    ///
    /// What the module says is checked as the module says it rather than by a pass that
    /// follows: a mistake that the surface alone decides is one a reader is told about before
    /// anything is built on top of it.
    fn function_rules(
        &mut self,
        decl: &FunDecl,
        function: &FunctionLoc,
        attributes: Attributes,
        visibility: Visibility,
    ) {
        self.repeated_parameters(decl, function);

        // An external function is implemented outside the project, so a body of it here is a
        // body that nothing would call, and the module says two things at once.
        if attributes.external
            && let Some(body) = decl.body()
        {
            let error = LoweringError::ExternFunctionHasBody {
                function: function.clone(),
            };
            let diagnostic = LoweringDiag::new(error, syntax::span(self.file, body.syntax()));

            self.diagnostics.push(diagnostic);
        }

        // A builtin is a function the compiler implements, and the declaration of it is what
        // puts its name in the scope of the module: a body here is one nothing calls either.
        if attributes.builtin
            && let Some(body) = decl.body()
        {
            let error = LoweringError::BuiltinFunctionHasBody {
                function: function.clone(),
            };
            let diagnostic = LoweringDiag::new(error, syntax::span(self.file, body.syntax()));

            self.diagnostics.push(diagnostic);
        }

        // A caller of a public function depends on the surface of its module, so the surface
        // has to say what the function takes and gives back ([ADR-0004]).
        //
        // [ADR-0004]: ../../docs/adr/0004-module-system.md
        if !visibility.is_public() {
            return;
        }

        let parameters = decl.parameters().ok();

        if decl.return_type_annotation().is_none() {
            // The annotation belongs where the parameters end, and an empty range there is
            // what a reader reads as the place to put it.
            let scope = parameters.as_ref().map_or_else(
                || decl.syntax().clone(),
                |parameters| parameters.syntax().clone(),
            );
            let error = LoweringError::PublicFunctionWithoutResult {
                function: function.clone(),
            };
            let diagnostic = LoweringDiag::new(error, syntax::after(self.file, &scope));

            self.diagnostics.push(diagnostic);
        }

        for parameter in decl::parameters(decl) {
            // A parameter the parser could not read is what the parse reported.
            let Some(parameter) = parameter else {
                continue;
            };

            if parameter.type_annotation().is_some() {
                continue;
            }

            let error = LoweringError::PublicParameterWithoutType {
                function: function.clone(),
                parameter: decl::parameter_name(&parameter),
            };
            let diagnostic = LoweringDiag::new(error, syntax::span(self.file, parameter.syntax()));

            self.diagnostics.push(diagnostic);
        }
    }

    /// Reports a parameter that binds a name another parameter of the function binds.
    ///
    /// A name a body reads is one name, and the parameters of a function are what the body
    /// binds it to: two parameters written under one name are two arguments a reader cannot
    /// tell apart, and the diagnostic is about the second of them, wherever in the list it is
    /// written. A parameter that binds no name is in the way of nothing.
    fn repeated_parameters(&mut self, decl: &FunDecl, function: &FunctionLoc) {
        let mut bound: Vec<Name> = Vec::new();

        for parameter in decl::parameters(decl) {
            // A parameter the parser could not read is what the parse reported.
            let Some(parameter) = parameter else {
                continue;
            };

            // A parameter that binds no name is in the way of nothing: a wildcard binds
            // nothing for a name to repeat.
            let Some(name) = decl::parameter_name(&parameter) else {
                continue;
            };

            if !bound.contains(&name) {
                bound.push(name);
                continue;
            }

            let error = LoweringError::DuplicateParameterName {
                function: function.clone(),
                name,
            };
            let diagnostic = LoweringDiag::new(error, syntax::span(self.file, parameter.syntax()));

            self.diagnostics.push(diagnostic);
        }
    }

    /// Declares one class.
    fn class(&mut self, decl: &TypeDecl, position: ItemSyntaxLoc) {
        let attributes = self.attributes(&decl.attributes());
        let visibility = decl::visibility(decl.visibility_token());

        let data = EntityData::Class(ClassData {
            attributes,
            visibility,
        });

        let loc = self.entity(
            decl.syntax(),
            position,
            Some(syntax::name(decl.name())),
            data,
        );
        let class = ClassLoc::try_from(loc.item).expect("a type is a type");

        // An external type is a type the project does not declare, and the language has not
        // decided how a module says that yet.
        if attributes.external {
            let error = LoweringError::ExternType { class };
            let diagnostic = LoweringDiag::new(error, syntax::span(self.file, decl.syntax()));

            self.diagnostics.push(diagnostic);
        }
    }

    /// Declares one import: what it brings in, and the name it is brought in under.
    fn import(&mut self, decl: &UseDecl, position: ItemSyntaxLoc) {
        // An import of nothing: a path the parser could not read is what the parse reported,
        // and there is no name for the import to bring in.
        let Ok(path) = decl.path() else {
            return;
        };

        let plain = path::plain(&path);
        self.plain_path(&path);

        // The name an import brings in is the name it is renamed to, and the last segment of
        // the path when it is not renamed: `use a.b.C as D` brings in `D`, and `use a.b.C`
        // brings in `C`.
        let alias = decl.alias().map(|alias| syntax::name(alias.name()));
        let name = alias
            .clone()
            .or_else(|| plain.last().cloned())
            .unwrap_or_else(Name::missing);

        let data = EntityData::Use(UseData {
            path: PlainPathId::new(plain),
            alias,
            visibility: decl::visibility(decl.visibility_token()),
        });

        self.entity(decl.syntax(), position, Some(name), data);
    }

    /// Reads the attributes a declaration writes in front of itself.
    ///
    /// The attributes the language has are the ones the HIR has fields for; a name that
    /// nothing knows is a mistake of the module, and the HIR has nowhere to put it. An
    /// attribute written twice is a mistake as well: what it says is what the first writing
    /// says, and the second one is not needed.
    fn attributes(&mut self, list: &AttributeList) -> Attributes {
        let mut attributes = Attributes::default();

        for attribute in decl::attributes(list) {
            let name = syntax::name(attribute.name());

            let error = if attributes.contains(&name) {
                LoweringError::RepeatedAttribute { name }
            } else if !attributes.insert(&name) {
                LoweringError::UnknownAttribute { name }
            } else {
                continue;
            };

            let diagnostic = LoweringDiag::new(error, syntax::span(self.file, attribute.syntax()));

            self.diagnostics.push(diagnostic);
        }

        attributes
    }

    /// Reports a path that carries type arguments where a path of the project belongs.
    ///
    /// A path of the project is a path of names, and what the arguments belong to is a type;
    /// the diagnostic points at the first of them rather than at the path, which is right
    /// about everything but its arguments.
    fn plain_path(&mut self, path: &mlkc_syntax::Path) {
        let Some(args) = path::type_args(path) else {
            return;
        };

        let error = LoweringError::PathIsNotPlain {
            path: path::plain(path),
        };
        let diagnostic = LoweringDiag::new(error, syntax::span(self.file, args.syntax()));

        self.diagnostics.push(diagnostic);
    }

    /// Declares one entity of the module.
    fn entity(
        &mut self,
        node: &SyntaxNode,
        position: ItemSyntaxLoc,
        name: Option<Name>,
        data: EntityData,
    ) -> EntityLoc {
        let declared = self.builder.declare(name, data, position);

        // The first declaration of a name in a namespace wins, and the one that lost is what a
        // diagnostic is about: the module declared the same name twice, and which of the two
        // its uses mean is not for a scope to decide. A name a module declares in another
        // namespace is not in the way of this one, and is not what this is about.
        if declared.duplicate {
            let error = LoweringError::DuplicateName {
                item: declared.loc.item.clone(),
            };
            let diagnostic = LoweringDiag::new(error, syntax::span(self.file, node));

            self.diagnostics.push(diagnostic);
        }

        declared.loc
    }
}

#[cfg(test)]
mod tests {
    use mlkc_diagnostics::{Category, DiagKind, Level};
    use mlkc_hir_def::{
        ClassLoc, FunctionLoc, ItemLoc, ItemLocLike, ItemTree, ModuleId, Namespace, PathAnchor,
    };
    use mlkc_vfs::FileId;

    use super::*;

    /// A module that is wrong in the ways the surface of a module can be wrong on its own.
    const SOURCE: &str = "\
@extern
type Handle

@extern
fun open(handle: Handle): Int =
    1

use std.core.Handle

use std.core.Int

use std.core.Int

@builtin
fun max(left: Int, right: Int): Int =
    left

@builtin
@builtin
type Point

pub fun size(value) =
    value

fun same(left: Int, left: Int): Int =
    left
";

    fn module() -> ModuleId {
        ModuleId(FileId::from_raw(0))
    }

    fn lower(source: &str) -> LoweredModule {
        let parsed = mlkc_parser::parse(source);
        let root = parsed.tree::<ModuleRoot>();

        crate::lower_module(module(), &root)
    }

    /// The name of the entity a name of the module denotes in `namespace`.
    fn item(tree: &ItemTree, namespace: Namespace, name: &str) -> ItemLoc {
        match tree.scope().anchor(&Name::new(name), namespace) {
            PathAnchor::Item(entity) => entity.item,
            anchor => panic!("`{name}` to be an entity of the module: {anchor:?}"),
        }
    }

    /// The name of the second declaration of a name, which is the one a duplicate is about.
    fn second(tree: &ItemTree, name: &str) -> ItemLoc {
        tree.entities()
            .map(|(loc, _)| loc)
            .find(|loc| loc.name() == Some(&Name::new(name)) && loc.disambiguator() == 1)
            .unwrap_or_else(|| panic!("a second declaration of `{name}`"))
    }

    fn class(tree: &ItemTree, name: &str) -> ClassLoc {
        ClassLoc::try_from(item(tree, Namespace::Ty, name)).expect("a class")
    }

    fn function(tree: &ItemTree, name: &str) -> FunctionLoc {
        FunctionLoc::try_from(item(tree, Namespace::Value, name)).expect("a function")
    }

    #[test]
    fn the_errors_of_a_module_are_typed_by_what_went_wrong() {
        let lowered = lower(SOURCE);
        let tree = &lowered.item_tree;

        let errors: Vec<&LoweringError> = lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.error())
            .collect();

        // The mistakes read in the order the module is written, whatever order the rules were
        // checked in: the two that a whole module decides --- an import of a declared name ---
        // are found last and read where they are.
        assert_eq!(errors, [
            &LoweringError::ExternType {
                class: class(tree, "Handle"),
            },
            &LoweringError::ExternFunctionHasBody {
                function: function(tree, "open"),
            },
            &LoweringError::ImportOfDeclaredName {
                name: Name::new("Handle"),
                declaration: EntityLoc {
                    module: module(),
                    item: item(tree, Namespace::Ty, "Handle"),
                },
            },
            &LoweringError::DuplicateName {
                item: second(tree, "Int"),
            },
            &LoweringError::BuiltinFunctionHasBody {
                function: function(tree, "max"),
            },
            &LoweringError::RepeatedAttribute {
                name: Name::new("builtin"),
            },
            &LoweringError::PublicParameterWithoutType {
                function: function(tree, "size"),
                parameter: Some(Name::new("value")),
            },
            &LoweringError::PublicFunctionWithoutResult {
                function: function(tree, "size"),
            },
            &LoweringError::DuplicateParameterName {
                function: function(tree, "same"),
                name: Name::new("left"),
            },
        ]);
    }

    #[test]
    fn an_error_of_a_kind_has_one_code_and_one_category() {
        let lowered = lower(SOURCE);

        let codes: Vec<&str> = lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.error().code())
            .collect();
        assert_eq!(codes, [
            "06", "05", "10", "01", "09", "11", "08", "07", "12"
        ]);

        // The error a host renders is the error, its kind, and where it is: nothing of it is
        // text that a caller has to parse back.
        let diagnostic = lowered.diagnostics[0].to_diagnostic();
        assert_eq!(diagnostic.level, Level::Error);
        assert_eq!(diagnostic.category, Category::Lowering);
        assert_eq!(diagnostic.code, "06");
        assert_eq!(diagnostic.labels.len(), 1);
        assert_eq!(diagnostic.labels[0].span, lowered.diagnostics[0].span());

        // A message is a rendering of the error, and the error is not the message: the text
        // of it says what the facts say and nothing of the kind or the code.
        assert!(
            diagnostic.message.contains("type Handle"),
            "{}",
            diagnostic.message,
        );
    }
}
