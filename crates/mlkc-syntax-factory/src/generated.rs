#[rustfmt::skip]
pub(super) mod syntax_factory;
// The factory is an API: the parser makes the nodes it needs, and a builder that no parse rule
// uses yet is not a mistake.
#[rustfmt::skip]
#[expect(
    dead_code,
    reason = "the generated factory is an API, and the parser does not construct every node yet"
)]
pub(super) mod node_factory;

pub use syntax_factory::SyntaxFactory;
