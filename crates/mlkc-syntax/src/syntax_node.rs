use mlkc_rowan::Language;

use crate::{ModuleRoot, SyntaxKind};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct MlkLanguage;

impl Language for MlkLanguage {
    type Kind = SyntaxKind;
    type Root = ModuleRoot;
}

pub type SyntaxNode = mlkc_rowan::SyntaxNode<MlkLanguage>;
pub type SyntaxToken = mlkc_rowan::SyntaxToken<MlkLanguage>;
pub type SyntaxElement = mlkc_rowan::SyntaxElement<MlkLanguage>;
pub type SyntaxNodeChildren = mlkc_rowan::SyntaxNodeChildren<MlkLanguage>;
pub type SyntaxElementChildren = mlkc_rowan::SyntaxElementChildren<MlkLanguage>;
pub type SyntaxList = mlkc_rowan::SyntaxList<MlkLanguage>;
pub type SyntaxTrivia = mlkc_rowan::syntax::SyntaxTrivia<MlkLanguage>;
pub type SyntaxNodeWithOffset = mlkc_rowan::syntax::SyntaxNodeWithOffset<MlkLanguage>;
pub type SyntaxNodePtr = mlkc_rowan::SyntaxNodePtr<MlkLanguage>;
