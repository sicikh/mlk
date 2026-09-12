use mlkc_rowan::{AstNode, ParsedChildren, RawNodeSlots, RawSyntaxNode, SyntaxFactory, SyntaxKind};

use crate::{MlkSyntaxKind, MlkSyntaxKind::*, *};

#[derive(Debug)]
pub struct MlkSyntaxFactory;

impl SyntaxFactory for MlkSyntaxFactory {
    type Kind = MlkSyntaxKind;

    fn make_syntax(
        kind: Self::Kind,
        children: ParsedChildren<Self::Kind>,
    ) -> RawSyntaxNode<Self::Kind> {
        match kind {
            BOGUS_EXPR => RawSyntaxNode::new(kind, children.into_iter().map(Some)),
            NUMBER_LITERAL_ATOM => {
                let mut elements = (&children).into_iter();
                let mut slots: RawNodeSlots<1usize> = RawNodeSlots::default();
                let mut current_element = elements.next();
                if let Some(element) = &current_element
                    && element.kind() == NUMBER_LITERAL
                {
                    slots.mark_present();
                    current_element = elements.next();
                }
                slots.next_slot();
                if current_element.is_some() {
                    return RawSyntaxNode::new(
                        NUMBER_LITERAL_ATOM.to_bogus(),
                        children.into_iter().map(Some),
                    );
                }
                slots.into_node(NUMBER_LITERAL_ATOM, children)
            },
            BOOLEAN_LITERAL_ATOM => {
                let mut elements = (&children).into_iter();
                let mut slots: RawNodeSlots<1usize> = RawNodeSlots::default();
                let mut current_element = elements.next();
                if let Some(element) = &current_element
                    && element.kind() == BOOLEAN_LITERAL
                {
                    slots.mark_present();
                    current_element = elements.next();
                }
                slots.next_slot();
                if current_element.is_some() {
                    return RawSyntaxNode::new(
                        BOOLEAN_LITERAL_ATOM.to_bogus(),
                        children.into_iter().map(Some),
                    );
                }
                slots.into_node(BOOLEAN_LITERAL_ATOM, children)
            },
            STRING_LITERAL_ATOM => {
                let mut elements = (&children).into_iter();
                let mut slots: RawNodeSlots<1usize> = RawNodeSlots::default();
                let mut current_element = elements.next();
                if let Some(element) = &current_element
                    && element.kind() == STRING_LITERAL
                {
                    slots.mark_present();
                    current_element = elements.next();
                }
                slots.next_slot();
                if current_element.is_some() {
                    return RawSyntaxNode::new(
                        STRING_LITERAL_ATOM.to_bogus(),
                        children.into_iter().map(Some),
                    );
                }
                slots.into_node(STRING_LITERAL_ATOM, children)
            },
            SYMBOL_ATOM => {
                let mut elements = (&children).into_iter();
                let mut slots: RawNodeSlots<1usize> = RawNodeSlots::default();
                let mut current_element = elements.next();
                if let Some(element) = &current_element
                    && element.kind() == IDENT
                {
                    slots.mark_present();
                    current_element = elements.next();
                }
                slots.next_slot();
                if current_element.is_some() {
                    return RawSyntaxNode::new(
                        SYMBOL_ATOM.to_bogus(),
                        children.into_iter().map(Some),
                    );
                }
                slots.into_node(SYMBOL_ATOM, children)
            },
            LIST_EXPR => {
                let mut elements = (&children).into_iter();
                let mut slots: RawNodeSlots<3usize> = RawNodeSlots::default();
                let mut current_element = elements.next();

                let paren_kind = if let Some(element) = &current_element
                    && matches!(element.kind(), L_PAREN | L_CURLY | R_BRACK)
                {
                    slots.mark_present();
                    let paren_kind = element.kind().rev_paren().unwrap();
                    current_element = elements.next();
                    Some(paren_kind)
                } else {
                    None
                };
                slots.next_slot();
                if let Some(element) = &current_element
                    && ExprList::can_cast(element.kind())
                {
                    slots.mark_present();
                    current_element = elements.next();
                }
                slots.next_slot();
                if let Some(element) = &current_element
                    && paren_kind.is_some_and(|kind| kind == element.kind())
                {
                    slots.mark_present();
                    current_element = elements.next();
                }
                slots.next_slot();
                if current_element.is_some() {
                    return RawSyntaxNode::new(
                        LIST_EXPR.to_bogus(),
                        children.into_iter().map(Some),
                    );
                }
                slots.into_node(LIST_EXPR, children)
            },
            ROOT_MODULE => {
                let mut elements = (&children).into_iter();
                let mut slots: RawNodeSlots<2usize> = RawNodeSlots::default();
                let mut current_element = elements.next();
                if let Some(element) = &current_element
                    && ExprList::can_cast(element.kind())
                {
                    slots.mark_present();
                    current_element = elements.next();
                }
                slots.next_slot();
                if let Some(element) = &current_element
                    && element.kind() == T![EOF]
                {
                    slots.mark_present();
                    current_element = elements.next();
                }
                slots.next_slot();
                if current_element.is_some() {
                    return RawSyntaxNode::new(
                        ROOT_MODULE.to_bogus(),
                        children.into_iter().map(Some),
                    );
                }
                slots.into_node(ROOT_MODULE, children)
            },
            EXPR_LIST => Self::make_node_list_syntax(kind, children, Expr::can_cast),
            _ => unreachable!("Is {:?} a token?", kind),
        }
    }
}
