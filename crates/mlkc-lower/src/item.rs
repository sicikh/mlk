//! The surface of one module: the items it declares, and the bodies they own.

use mlkc_hir_def::{
    Attributes, BodyEntityLoc, ClassData, EntityData, EntityLoc, FunctionData, ItemSyntaxLoc,
    ItemTreeBuilder, ModuleId, Name, PlainPathId,
};
use mlkc_rowan::AstNode;
use mlkc_syntax::{AttributeList, FunDecl, ModuleItem, ModuleRoot, SyntaxNode, TypeDecl};
use mlkc_vfs::FileId;

use crate::{
    BodyDecl, LoweredModule, LoweringDiag, decl, path,
    syntax::{self, item_position},
};

/// Lowers the items of `root` into the item tree of `module`.
pub(crate) fn lower(module: ModuleId, root: &ModuleRoot) -> LoweredModule {
    let mut lowering = ItemLowering {
        module,
        file: module.0,
        builder: ItemTreeBuilder::new(module),
        diagnostics: Vec::new(),
        bodies: Vec::new(),
    };

    lowering.preamble(root);
    lowering.items(root);

    let ItemLowering {
        builder,
        diagnostics,
        bodies,
        ..
    } = lowering;

    LoweredModule {
        item_tree: builder.finish(),
        diagnostics,
        bodies,
    }
}

/// The lowering of the items of one module.
struct ItemLowering {
    module: ModuleId,
    file: FileId,
    builder: ItemTreeBuilder,
    diagnostics: Vec<LoweringDiag>,
    bodies: Vec<BodyDecl>,
}

impl ItemLowering {
    /// Records the path the module declares itself as, which is what its preamble writes.
    ///
    /// A module that has no preamble declares no path: what it is called is what the project
    /// it belongs to says, and the path of the file is the canonical form of that.
    fn preamble(&mut self, root: &ModuleRoot) {
        let Some(preamble) = root.preamble() else {
            return;
        };

        // A preamble whose path the parser could not read declares nothing:
        // a path of no segments names nothing, and the parse is what reported the mistake.
        let Ok(path) = preamble.name() else {
            return;
        };

        self.builder.set_path(PlainPathId::new(path::plain(&path)));
    }

    /// Declares every item of the module, in the order it is written.
    fn items(&mut self, root: &ModuleRoot) {
        let list = root.items().syntax().clone();

        for node in list.children() {
            let position = item_position(&list, &node);

            // A declaration the parser could not read is what the parse reported, and the
            // HIR holds what the module does say: a broken item says nothing.
            match ModuleItem::cast(node) {
                Some(ModuleItem::FunDecl(decl)) => self.function(&decl, position),
                Some(ModuleItem::TypeDecl(decl)) => self.class(&decl, position),
                Some(ModuleItem::BogusDecl(_)) | None => {},
            }
        }
    }

    /// Declares one function, and remembers the body it owns.
    fn function(&mut self, decl: &FunDecl, position: ItemSyntaxLoc) {
        let data = EntityData::Function(FunctionData {
            attributes: self.attributes(&decl.attributes()),
            visibility: decl::visibility(decl.visibility_token()),
            signature: decl::signature(decl),
        });

        let loc = self.entity(
            decl.syntax(),
            position,
            Some(syntax::name(decl.name())),
            data,
        );

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

    /// Declares one class.
    fn class(&mut self, decl: &TypeDecl, position: ItemSyntaxLoc) {
        let data = EntityData::Class(ClassData {
            attributes: self.attributes(&decl.attributes()),
            visibility: decl::visibility(decl.visibility_token()),
        });

        self.entity(
            decl.syntax(),
            position,
            Some(syntax::name(decl.name())),
            data,
        );
    }

    /// Reads the attributes a declaration writes in front of itself.
    ///
    /// The attributes the language has are the ones the HIR has fields for; a name that
    /// nothing knows is a mistake of the module, and the HIR has nowhere to put it.
    fn attributes(&mut self, list: &AttributeList) -> Attributes {
        let mut attributes = Attributes::default();

        for attribute in decl::attributes(list) {
            let name = syntax::name(attribute.name());

            if !attributes.insert(&name) {
                let message = format!("the language has no attribute `{name:?}`");
                let diagnostic =
                    LoweringDiag::new(message, syntax::span(self.file, attribute.syntax()));

                self.diagnostics.push(diagnostic);
            }
        }

        attributes
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
            let message = format!(
                "`{:?}` is declared more than once in this module",
                declared.loc.item,
            );
            let diagnostic = LoweringDiag::new(message, syntax::span(self.file, node));

            self.diagnostics.push(diagnostic);
        }

        declared.loc
    }
}
