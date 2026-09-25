//! Reading the syntax tree the way the HIR is written down.
//!
//! The lowering walks the tree by its children and casts what it finds, rather than walking
//! a typed list: a tree that a mistake broke is read up to the mistake, a token where a node
//! belongs is skipped, and nothing panics over what a reader of the source typed.
//!
//! The names and the positions the HIR is made of are read here too, once, so that the item
//! tree and the bodies read a declaration the same way.

use mlkc_hir_def::{ItemSyntaxLoc, Name};
use mlkc_rowan::{AstNode, SyntaxResult};
use mlkc_span::Span;
use mlkc_syntax::{ModuleRoot, Name as NameSyntax, SyntaxNode, TextRange};
use mlkc_vfs::FileId;

/// The name a declaration or a reference is written with, or a name that is not there.
///
/// A name the parser could not read is [`Name::missing`], not an error: the item that
/// carries it is still an item of the module, and a reader of the HIR asks a name whether it
/// is there instead of holding an `Option` at every use.
pub(crate) fn name(name: SyntaxResult<NameSyntax>) -> Name {
    let Some(name) = name.ok() else {
        return Name::missing();
    };
    let Some(token) = name.value_token().ok() else {
        return Name::missing();
    };

    Name::new(token.text_trimmed())
}

/// The position of an item in the module: the list of the items, then the item inside it.
pub(crate) fn item_position(list: &SyntaxNode, item: &SyntaxNode) -> ItemSyntaxLoc {
    let mut position = ItemSyntaxLoc::root();

    position.push(slot(list.index()));
    position.push(slot(item.index()));

    position
}

/// Where a node is in the file it was read from, without the trivia around it.
pub(crate) fn span(file: FileId, node: &SyntaxNode) -> Span {
    Span::new(file, node.text_trimmed_range())
}

/// Where a thing that is not there would be written: right after `node`.
///
/// A diagnostic about what a module did not write points at the place it belongs rather than
/// at the whole declaration: a reader reads an empty range as the place to put it.
pub(crate) fn after(file: FileId, node: &SyntaxNode) -> Span {
    Span::new(file, TextRange::empty(node.text_trimmed_range().end()))
}

/// The syntax at a position of an item tree.
///
/// A position is a path of child slots from the module root, which is what the lowering
/// records when it declares an entity, so this is the walk back: a caller that holds an item
/// tree and the syntax it was lowered from gets the declaration of any entity of it --- to
/// lower its body, to mark it, or to describe it.
///
/// A position the tree does not have --- because it was recorded in another revision of the
/// module --- is `None`.
pub fn syntax_at(root: &ModuleRoot, position: &ItemSyntaxLoc) -> Option<SyntaxNode> {
    let mut node = root.syntax().clone();

    for slot in position.slots() {
        node = child_at(&node, *slot)?;
    }

    Some(node)
}

/// The child of `node` that lives in `slot`.
fn child_at(node: &SyntaxNode, slot: u32) -> Option<SyntaxNode> {
    node.children().find(|child| slot_of(child) == slot)
}

/// The slot a node lives in inside its parent.
fn slot_of(node: &SyntaxNode) -> u32 {
    slot(node.index())
}

/// A position in a tree, as a slot of a node.
///
/// A file is never this deep, and a position that saturates is a position that is not there,
/// which is what a caller reads: not worth an error, and not worth a panic.
fn slot(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}
