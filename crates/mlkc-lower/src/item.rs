//! The surface of one module: the items it declares, and the bodies they own.

use mlkc_hir_def::{
    BodyEntityLoc, ClassData, EntityData, EntityLoc, FunctionData, ItemLocLike, ItemSyntaxLoc,
    ItemTreeBuilder, ModuleId, Name, Visibility,
};
use mlkc_rowan::AstNode;
use mlkc_syntax::{FunDecl, ModuleItem, ModuleRoot, SyntaxNode, TypeDecl};
use mlkc_vfs::FileId;

use crate::{
    BodyDecl, LoweredModule, LoweringDiag, decl,
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
            visibility: Visibility::Private,
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
            visibility: Visibility::Private,
        });

        self.entity(
            decl.syntax(),
            position,
            Some(syntax::name(decl.name())),
            data,
        );
    }

    /// Declares one entity of the module.
    fn entity(
        &mut self,
        node: &SyntaxNode,
        position: ItemSyntaxLoc,
        name: Option<Name>,
        data: EntityData,
    ) -> EntityLoc {
        // A name that is not there is not a name the module declared twice, however many
        // entities are missing one: the parse reports the missing name, and there is nothing
        // for a duplicate to be about.
        let named = name.as_ref().is_some_and(|name| !name.is_missing());
        let loc = self.builder.declare(name, data, position);

        // The first declaration of a name wins, and the one that lost is what a diagnostic
        // is about: the module declared the same name twice, and which of the two its uses
        // mean is not for a scope to decide.
        if named && loc.item.disambiguator() > 0 {
            let message = format!("`{:?}` is declared more than once in this module", loc.item);
            let diagnostic = LoweringDiag::new(message, syntax::span(self.file, node));

            self.diagnostics.push(diagnostic);
        }

        loc
    }
}
