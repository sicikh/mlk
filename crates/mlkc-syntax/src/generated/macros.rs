//! Generated file, do not edit by hand, see `xtask/codegen`

#[doc = r" Reconstruct an AstNode from a SyntaxNode"]
#[doc = r""]
#[doc = r" This macros performs a match over the [kind](mlkc_rowan::SyntaxNode::kind)"]
#[doc = r" of the provided [mlkc_rowan::SyntaxNode] and constructs the appropriate"]
#[doc = r" AstNode type for it, then execute the provided expression over it."]
#[doc = r""]
#[doc = r" # Examples"]
#[doc = r""]
#[doc = r" ```ignore"]
#[doc = r" map_syntax_node!(syntax_node, node => node.format())"]
#[doc = r" ```"]
#[macro_export]
macro_rules! map_syntax_node {
    ($node:expr, $pattern:pat => $body:expr) => {
        match $node {
            node => {
                match $crate::SyntaxNode::kind(&node) {
                    $crate::SyntaxKind::ATTRIBUTE => {
                        let $pattern = unsafe { $crate::Attribute::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::BIN_EXPR => {
                        let $pattern = unsafe { $crate::BinExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::CALL_EXPR => {
                        let $pattern = unsafe { $crate::CallExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::FUN_BODY => {
                        let $pattern = unsafe { $crate::FunBody::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::FUN_DECL => {
                        let $pattern = unsafe { $crate::FunDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::FUN_RETURN_TYPE_ANNOTATION => {
                        let $pattern =
                            unsafe { $crate::FunReturnTypeAnnotation::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::IDENT_PAT => {
                        let $pattern = unsafe { $crate::IdentPat::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::INFER_TYPE => {
                        let $pattern = unsafe { $crate::InferType::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::INT_LITERAL => {
                        let $pattern = unsafe { $crate::IntLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::LET_EXPR => {
                        let $pattern = unsafe { $crate::LetExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::MODULE_PREAMBLE => {
                        let $pattern = unsafe { $crate::ModulePreamble::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::MODULE_ROOT => {
                        let $pattern = unsafe { $crate::ModuleRoot::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::NAME => {
                        let $pattern = unsafe { $crate::Name::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::PARAMETER => {
                        let $pattern = unsafe { $crate::Parameter::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::PARAMETERS => {
                        let $pattern = unsafe { $crate::Parameters::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::PAREN_EXPR => {
                        let $pattern = unsafe { $crate::ParenExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::PATH => {
                        let $pattern = unsafe { $crate::Path::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::PATH_QUALIFIER => {
                        let $pattern = unsafe { $crate::PathQualifier::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::PATH_SEGMENT => {
                        let $pattern = unsafe { $crate::PathSegment::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::PATH_TYPE => {
                        let $pattern = unsafe { $crate::PathType::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::STRING_LITERAL => {
                        let $pattern = unsafe { $crate::StringLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::TYPE_ANNOTATION => {
                        let $pattern = unsafe { $crate::TypeAnnotation::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::TYPE_ARGS => {
                        let $pattern = unsafe { $crate::TypeArgs::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::TYPE_DECL => {
                        let $pattern = unsafe { $crate::TypeDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::USE_ALIAS => {
                        let $pattern = unsafe { $crate::UseAlias::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::USE_DECL => {
                        let $pattern = unsafe { $crate::UseDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::VAR_EXPR => {
                        let $pattern = unsafe { $crate::VarExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::WILDCARD_PAT => {
                        let $pattern = unsafe { $crate::WildcardPat::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::BOGUS => {
                        let $pattern = unsafe { $crate::Bogus::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::BOGUS_DECL => {
                        let $pattern = unsafe { $crate::BogusDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::BOGUS_EXPR => {
                        let $pattern = unsafe { $crate::BogusExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::BOGUS_PARAMETER => {
                        let $pattern = unsafe { $crate::BogusParameter::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::BOGUS_PAT => {
                        let $pattern = unsafe { $crate::BogusPat::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::BOGUS_TYPE => {
                        let $pattern = unsafe { $crate::BogusType::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::ARGUMENT_LIST => {
                        let $pattern = unsafe { $crate::ArgumentList::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::ATTRIBUTE_LIST => {
                        let $pattern = unsafe { $crate::AttributeList::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::MODULE_ITEM_LIST => {
                        let $pattern = unsafe { $crate::ModuleItemList::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::PARAMETER_LIST => {
                        let $pattern = unsafe { $crate::ParameterList::new_unchecked(node) };
                        $body
                    },
                    $crate::SyntaxKind::TYPE_ARG_LIST => {
                        let $pattern = unsafe { $crate::TypeArgList::new_unchecked(node) };
                        $body
                    },
                    _ => unreachable!(),
                }
            },
        }
    };
}
pub(crate) use map_syntax_node;
