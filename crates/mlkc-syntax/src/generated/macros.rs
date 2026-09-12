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
                    $crate::MlkSyntaxKind::AND_PAT => {
                        let $pattern = unsafe { $crate::AndPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::APP_EXPR => {
                        let $pattern = unsafe { $crate::AppExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::AS_PAT => {
                        let $pattern = unsafe { $crate::AsPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BIN_EXPR => {
                        let $pattern = unsafe { $crate::BinExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BINDING => {
                        let $pattern = unsafe { $crate::Binding::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::BOOL_LITERAL => {
                        let $pattern = unsafe { $crate::BoolLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::CHAR_LITERAL => {
                        let $pattern = unsafe { $crate::CharLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::CONS_PAT => {
                        let $pattern = unsafe { $crate::ConsPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::FN_TY => {
                        let $pattern = unsafe { $crate::FnTy::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::FUN_EXPR => {
                        let $pattern = unsafe { $crate::FunExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::FUNC_PAT => {
                        let $pattern = unsafe { $crate::FuncPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::IF_EXPR => {
                        let $pattern = unsafe { $crate::IfExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::INFER_TY => {
                        let $pattern = unsafe { $crate::InferTy::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::INNER_MODULE_ITEM => {
                        let $pattern = unsafe { $crate::InnerModuleItem::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::INT_LITERAL => {
                        let $pattern = unsafe { $crate::IntLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LET_DECL => {
                        let $pattern = unsafe { $crate::LetDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LET_EXPR => {
                        let $pattern = unsafe { $crate::LetExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LIST_EXPR => {
                        let $pattern = unsafe { $crate::ListExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LIST_PAT => {
                        let $pattern = unsafe { $crate::ListPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LITERAL => {
                        let $pattern = unsafe { $crate::Literal::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LITERAL_PAT => {
                        let $pattern = unsafe { $crate::LiteralPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MATCH_CASE => {
                        let $pattern = unsafe { $crate::MatchCase::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MATCH_EXPR => {
                        let $pattern = unsafe { $crate::MatchExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MATCH_GUARD => {
                        let $pattern = unsafe { $crate::MatchGuard::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MEMBER_ACCESS_EXPR => {
                        let $pattern = unsafe { $crate::MemberAccessExpr::new_unchecked(node) };
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
                    $crate::MlkSyntaxKind::NAME_PAT_FIELD => {
                        let $pattern = unsafe { $crate::NamePatField::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::NAMED_PAT => {
                        let $pattern = unsafe { $crate::NamedPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::OPEN_DECL => {
                        let $pattern = unsafe { $crate::OpenDecl::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::OPERATOR => {
                        let $pattern = unsafe { $crate::Operator::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::OR_PAT => {
                        let $pattern = unsafe { $crate::OrPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PAREN_EXPR => {
                        let $pattern = unsafe { $crate::ParenExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PAREN_PAT => {
                        let $pattern = unsafe { $crate::ParenPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::PAREN_TY => {
                        let $pattern = unsafe { $crate::ParenTy::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::Q_NAME => {
                        let $pattern = unsafe { $crate::QName::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::Q_NAME_SEGMENT => {
                        let $pattern = unsafe { $crate::QNameSegment::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::Q_TY => {
                        let $pattern = unsafe { $crate::QTy::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::RECORD_PAT => {
                        let $pattern = unsafe { $crate::RecordPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::SEQ_EXPR => {
                        let $pattern = unsafe { $crate::SeqExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::STRING_LITERAL => {
                        let $pattern = unsafe { $crate::StringLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TYPED_EXPR => {
                        let $pattern = unsafe { $crate::TypedExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TYPED_PAT => {
                        let $pattern = unsafe { $crate::TypedPat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::UNARY_EXPR => {
                        let $pattern = unsafe { $crate::UnaryExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::UNIT_LITERAL => {
                        let $pattern = unsafe { $crate::UnitLiteral::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::VAR_EXPR => {
                        let $pattern = unsafe { $crate::VarExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::WILD_PAT => {
                        let $pattern = unsafe { $crate::WildPat::new_unchecked(node) };
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
                    $crate::MlkSyntaxKind::BOGUS_TY => {
                        let $pattern = unsafe { $crate::BogusTy::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::ARG_PATS => {
                        let $pattern = unsafe { $crate::ArgPats::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LIST_EXPR_ELEMENTS => {
                        let $pattern = unsafe { $crate::ListExprElements::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::LIST_PAT_ELEMENTS => {
                        let $pattern = unsafe { $crate::ListPatElements::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MATCH_CASE_LIST => {
                        let $pattern = unsafe { $crate::MatchCaseList::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::MODULE_ITEM_LIST => {
                        let $pattern = unsafe { $crate::ModuleItemList::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::RECORD_FIELDS => {
                        let $pattern = unsafe { $crate::RecordFields::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TUPLE_EXPR => {
                        let $pattern = unsafe { $crate::TupleExpr::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TUPLE_PAT => {
                        let $pattern = unsafe { $crate::TuplePat::new_unchecked(node) };
                        $body
                    },
                    $crate::MlkSyntaxKind::TUPLE_TY => {
                        let $pattern = unsafe { $crate::TupleTy::new_unchecked(node) };
                        $body
                    },
                    _ => unreachable!(),
                }
            },
        }
    };
}
pub(crate) use map_syntax_node;
