use mlkc_rowan::TreeBuilder;
use mlkc_syntax::MlkLanguage;

mod generated;
pub use crate::generated::SyntaxFactory;

pub type SyntaxTreeBuilder = TreeBuilder<'static, MlkLanguage, SyntaxFactory>;
