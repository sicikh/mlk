//! Generated file, do not edit by hand, see `xtask/codegen`

#[doc = r" Reconstruct an AstNode from a SyntaxNode"]
#[doc = r""]
#[doc = r" This macros performs a match over the [kind](biome_rowan::SyntaxNode::kind)"]
#[doc = r" of the provided [biome_rowan::SyntaxNode] and constructs the appropriate"]
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
                match $crate::MlkSyntaxNode::kind(&node) {
                    $crate::MlkSyntaxKind::ATTRIBUTE => {
                        let $pattern = unsafe { $crate::Attribute::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BIN_EXPR => {
                        let $pattern = unsafe { $crate::BinExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::CALL_EXPR => {
                        let $pattern = unsafe { $crate::CallExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::FUN_BODY => {
                        let $pattern = unsafe { $crate::FunBody::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::FUN_DECL => {
                        let $pattern = unsafe { $crate::FunDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::FUN_RETURN_TYPE_ANNOTATION => {
                        let $pattern =
                            unsafe { $crate::FunReturnTypeAnnotation::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::IDENT_PAT => {
                        let $pattern = unsafe { $crate::IdentPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::INFER_TYPE => {
                        let $pattern = unsafe { $crate::InferType::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::INT_LITERAL => {
                        let $pattern = unsafe { $crate::IntLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LET_EXPR => {
                        let $pattern = unsafe { $crate::LetExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MODULE_PREAMBLE => {
                        let $pattern = unsafe { $crate::ModulePreamble::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MODULE_ROOT => {
                        let $pattern = unsafe { $crate::ModuleRoot::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::NAME => {
                        let $pattern = unsafe { $crate::Name::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PARAMETER => {
                        let $pattern = unsafe { $crate::Parameter::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PARAMETERS => {
                        let $pattern = unsafe { $crate::Parameters::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PAREN_EXPR => {
                        let $pattern = unsafe { $crate::ParenExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PATH => {
                        let $pattern = unsafe { $crate::Path::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PATH_SEGMENT => {
                        let $pattern = unsafe { $crate::PathSegment::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PATH_TYPE => {
                        let $pattern = unsafe { $crate::PathType::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::STRING_LITERAL => {
                        let $pattern = unsafe { $crate::StringLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TYPE_ANNOTATION => {
                        let $pattern = unsafe { $crate::TypeAnnotation::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TYPE_ARGS => {
                        let $pattern = unsafe { $crate::TypeArgs::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TYPE_DECL => {
                        let $pattern = unsafe { $crate::TypeDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::VAR_EXPR => {
                        let $pattern = unsafe { $crate::VarExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::WILDCARD_PAT => {
                        let $pattern = unsafe { $crate::WildcardPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BOGUS => {
                        let $pattern = unsafe { $crate::Bogus::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BOGUS_DECL => {
                        let $pattern = unsafe { $crate::BogusDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BOGUS_EXPR => {
                        let $pattern = unsafe { $crate::BogusExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BOGUS_PAT => {
                        let $pattern = unsafe { $crate::BogusPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BOGUS_TYPE => {
                        let $pattern = unsafe { $crate::BogusType::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::ARGUMENT_LIST => {
                        let $pattern = unsafe { $crate::ArgumentList::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::ATTRIBUTE_LIST => {
                        let $pattern = unsafe { $crate::AttributeList::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MODULE_ITEM_LIST => {
                        let $pattern = unsafe { $crate::ModuleItemList::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PARAMETER_LIST => {
                        let $pattern = unsafe { $crate::ParameterList::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TYPE_ARG_LIST => {
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
