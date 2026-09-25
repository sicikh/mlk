//! Generated file, do not edit by hand, see `xtask/codegen`

use std::iter::once;

use mlkc_rowan::AstNode;

use crate::{SyntaxToken, generated::nodes::*};
impl Attribute {
    pub fn with_at_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_name(self, element: Name) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl BinExpr {
    pub fn with_lhs(self, element: Expr) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_operator_token_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into()))),
        )
    }
    pub fn with_rhs(self, element: Expr) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl CallExpr {
    pub fn with_function(self, element: Expr) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_l_paren_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into()))),
        )
    }
    pub fn with_arguments(self, element: ArgumentList) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_r_paren_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(3usize..=3usize, once(Some(element.into()))),
        )
    }
}
impl FunBody {
    pub fn with_eq_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_expr(self, element: Expr) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl FunDecl {
    pub fn with_attributes(self, element: AttributeList) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_visibility_token(self, element: Option<SyntaxToken>) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(element.map(|element| element.into()))),
        )
    }
    pub fn with_fun_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into()))),
        )
    }
    pub fn with_name(self, element: Name) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(3usize..=3usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_parameters(self, element: Parameters) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(4usize..=4usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_return_type_annotation(self, element: Option<FunReturnTypeAnnotation>) -> Self {
        Self::unwrap_cast(self.syntax.splice_slots(
            5usize..=5usize,
            once(element.map(|element| element.into_syntax().into())),
        ))
    }
    pub fn with_body(self, element: Option<FunBody>) -> Self {
        Self::unwrap_cast(self.syntax.splice_slots(
            6usize..=6usize,
            once(element.map(|element| element.into_syntax().into())),
        ))
    }
}
impl FunReturnTypeAnnotation {
    pub fn with_colon_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_return_type(self, element: Type) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl IdentPat {
    pub fn with_name(self, element: Name) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl InferType {
    pub fn with_underscore_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
}
impl IntLiteral {
    pub fn with_value_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
}
impl LetExpr {
    pub fn with_let_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_pat(self, element: Pat) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_eq_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into()))),
        )
    }
    pub fn with_expr(self, element: Expr) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(3usize..=3usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_in_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(4usize..=4usize, once(Some(element.into()))),
        )
    }
    pub fn with_body(self, element: Expr) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(5usize..=5usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl ModulePreamble {
    pub fn with_module_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_name(self, element: Path) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl ModuleRoot {
    pub fn with_bom_token(self, element: Option<SyntaxToken>) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(element.map(|element| element.into()))),
        )
    }
    pub fn with_preamble(self, element: Option<ModulePreamble>) -> Self {
        Self::unwrap_cast(self.syntax.splice_slots(
            1usize..=1usize,
            once(element.map(|element| element.into_syntax().into())),
        ))
    }
    pub fn with_items(self, element: ModuleItemList) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_eof_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(3usize..=3usize, once(Some(element.into()))),
        )
    }
}
impl Name {
    pub fn with_value_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
}
impl Parameter {
    pub fn with_pat(self, element: Pat) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_type_annotation(self, element: Option<TypeAnnotation>) -> Self {
        Self::unwrap_cast(self.syntax.splice_slots(
            1usize..=1usize,
            once(element.map(|element| element.into_syntax().into())),
        ))
    }
}
impl Parameters {
    pub fn with_l_paren_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_items(self, element: ParameterList) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_r_paren_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into()))),
        )
    }
}
impl ParenExpr {
    pub fn with_l_paren_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_expr(self, element: Expr) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_r_paren_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into()))),
        )
    }
}
impl Path {
    pub fn with_qualifier(self, element: Option<PathQualifier>) -> Self {
        Self::unwrap_cast(self.syntax.splice_slots(
            0usize..=0usize,
            once(element.map(|element| element.into_syntax().into())),
        ))
    }
    pub fn with_segment(self, element: PathSegment) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl PathExpr {
    pub fn with_path(self, element: Path) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl PathQualifier {
    pub fn with_path(self, element: Path) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_dot_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into()))),
        )
    }
}
impl PathSegment {
    pub fn with_name(self, element: Name) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_type_args(self, element: Option<TypeArgs>) -> Self {
        Self::unwrap_cast(self.syntax.splice_slots(
            1usize..=1usize,
            once(element.map(|element| element.into_syntax().into())),
        ))
    }
}
impl PathType {
    pub fn with_path(self, element: Path) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl StringLiteral {
    pub fn with_value_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
}
impl TypeAnnotation {
    pub fn with_colon_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_ty(self, element: Type) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl TypeArgs {
    pub fn with_l_brack_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_type_arg_list(self, element: TypeArgList) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_r_brack_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into()))),
        )
    }
}
impl TypeDecl {
    pub fn with_attributes(self, element: AttributeList) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_visibility_token(self, element: Option<SyntaxToken>) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(element.map(|element| element.into()))),
        )
    }
    pub fn with_type_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into()))),
        )
    }
    pub fn with_name(self, element: Name) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(3usize..=3usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl UnaryExpr {
    pub fn with_operator_token_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_operand(self, element: Expr) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl UseAlias {
    pub fn with_as_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
    pub fn with_name(self, element: Name) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into_syntax().into()))),
        )
    }
}
impl UseDecl {
    pub fn with_visibility_token(self, element: Option<SyntaxToken>) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(element.map(|element| element.into()))),
        )
    }
    pub fn with_use_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(1usize..=1usize, once(Some(element.into()))),
        )
    }
    pub fn with_path(self, element: Path) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(2usize..=2usize, once(Some(element.into_syntax().into()))),
        )
    }
    pub fn with_alias(self, element: Option<UseAlias>) -> Self {
        Self::unwrap_cast(self.syntax.splice_slots(
            3usize..=3usize,
            once(element.map(|element| element.into_syntax().into())),
        ))
    }
}
impl WildcardPat {
    pub fn with_underscore_token(self, element: SyntaxToken) -> Self {
        Self::unwrap_cast(
            self.syntax
                .splice_slots(0usize..=0usize, once(Some(element.into()))),
        )
    }
}
