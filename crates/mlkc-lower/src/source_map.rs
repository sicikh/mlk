//! Where the nodes of a body are written.
//!
//! The HIR holds no positions: a body is values, and a range is what a host that marks a buffer
//! with reads. The lowering is the only stage that knows which syntax a node was read from, so
//! the map is built as the body is, and handed out with it.
//!
//! A range is the range of the syntax the node was read from, with the trivia around it left
//! out. A node the lowering made up rather than read --- an expression the parser did not find
//! --- has no range at all, and a parenthesized expression reads as the expression it holds,
//! parentheses included.

use mlkc_hir_def::{ExprId, PatId, PathId};
use mlkc_la_arena::ArenaMap;
use mlkc_syntax::TextRange;

/// Where the nodes of one body are written, by the id of the node.
///
/// A map is a value of one lowering of one body: the ids are the positions of the arenas of
/// that body, and a range is a place in the file it was read from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BodySourceMap {
    exprs: ArenaMap<ExprId, TextRange>,
    pats: ArenaMap<PatId, TextRange>,
    paths: ArenaMap<PathId, TextRange>,
}

impl BodySourceMap {
    /// Where the expression with this id is written, if the lowering read it from the source.
    pub fn expr(&self, id: ExprId) -> Option<TextRange> {
        self.exprs.get(id).copied()
    }

    /// Where the pattern with this id is written, if the lowering read it from the source.
    pub fn pat(&self, id: PatId) -> Option<TextRange> {
        self.pats.get(id).copied()
    }

    /// Where the path with this id is written, if the lowering read it from the source.
    pub fn path(&self, id: PathId) -> Option<TextRange> {
        self.paths.get(id).copied()
    }

    /// Records where an expression is written.
    ///
    /// A range written twice is the last one: a parenthesized expression is the expression it
    /// holds, and what a reader marks for it is the parentheses as well.
    pub(crate) fn set_expr(&mut self, id: ExprId, range: TextRange) {
        self.exprs.insert(id, range);
    }

    /// Records where a pattern is written.
    pub(crate) fn set_pat(&mut self, id: PatId, range: TextRange) {
        self.pats.insert(id, range);
    }

    /// Records where a path is written.
    pub(crate) fn set_path(&mut self, id: PathId, range: TextRange) {
        self.paths.insert(id, range);
    }
}

#[cfg(test)]
mod tests {
    use mlkc_hir_def::{Expr, ModuleId, Pat, PathAnchor, Prelude};
    use mlkc_syntax::{ModuleRoot, TextRange, TextSize};
    use mlkc_vfs::FileId;

    use crate::{LoweredBody, lower_body, lower_module};

    /// The file a fixture is lowered as.
    const FILE: FileId = FileId::from_raw(0);

    /// The range `text` is written at in `source`.
    fn at(source: &str, text: &str) -> TextRange {
        let start = source
            .find(text)
            .unwrap_or_else(|| panic!("{text:?} is not in the source"));

        TextRange::at(TextSize::of(&source[..start]), TextSize::of(text))
    }

    /// The lowering of the body of the first function of `source`.
    fn body_of(source: &str) -> LoweredBody {
        let parsed = mlkc_parser::parse(source);
        let root = parsed.tree::<ModuleRoot>();
        let module = lower_module(ModuleId(FILE), &root, &Prelude::none());
        let decl = module.bodies.first().expect("a function with a body");

        lower_body(&module.item_tree, &decl.decl).expect("a body to be lowered")
    }

    #[test]
    fn a_node_read_from_the_source_is_a_range_of_it() {
        let source = "fun main(): Int =\n    let x = 1 in\n    x + 2\n";
        let lowered = body_of(source);
        let body = &lowered.body;

        let Expr::Let {
            pat,
            expr,
            body: inner,
        } = &body[body.root()]
        else {
            panic!("a `let` is the root of the body");
        };

        assert_eq!(lowered.source_map.pat(*pat), Some(at(source, "x")));
        assert_eq!(lowered.source_map.expr(*expr), Some(at(source, "1")));
        assert_eq!(lowered.source_map.expr(*inner), Some(at(source, "x + 2")));
    }

    #[test]
    fn a_path_is_the_name_it_was_written_as() {
        let source = "fun main(): Int =\n    println-int(1)\n";
        let lowered = body_of(source);
        let body = &lowered.body;

        let Expr::Call { callee, args } = &body[body.root()] else {
            panic!("a call is the root of the body");
        };

        let Expr::Path(path) = &body[*callee] else {
            panic!("the callee is a path");
        };

        assert_eq!(
            lowered.source_map.expr(*callee),
            Some(at(source, "println-int"))
        );
        assert_eq!(
            lowered.source_map.path(*path),
            Some(at(source, "println-int"))
        );
        assert_eq!(
            lowered
                .source_map
                .expr(*args.first().expect("one argument")),
            Some(at(source, "1")),
        );
    }

    #[test]
    fn an_expression_the_lowering_made_up_is_written_nowhere() {
        let source = "fun main(): Int =\n";
        let lowered = body_of(source);

        assert_eq!(lowered.body[lowered.body.root()], Expr::Missing);
        assert_eq!(lowered.source_map.expr(lowered.body.root()), None);
    }

    #[test]
    fn a_call_written_with_a_dot_reads_as_the_call_it_means() {
        let source = "fun main(): Int =\n    value.function(1)\n";
        let lowered = body_of(source);
        let body = &lowered.body;

        let Expr::Call { callee, args } = &body[body.root()] else {
            panic!("a call is the root of the body");
        };

        // The call is about the whole spelling, the callee is the name written after the dot,
        // and the receiver is the first argument, where it is written.
        assert_eq!(
            lowered.source_map.expr(body.root()),
            Some(at(source, "value.function(1)")),
        );
        assert_eq!(
            lowered.source_map.expr(*callee),
            Some(at(source, "function"))
        );
        assert_eq!(lowered.source_map.expr(args[0]), Some(at(source, "value")));
        assert_eq!(lowered.source_map.expr(args[1]), Some(at(source, "1")));
    }

    #[test]
    fn a_call_written_with_a_dot_may_be_named_by_a_path() {
        let source = "fun main(): Int =\n    value.Map::insert(name)\n";
        let lowered = body_of(source);
        let body = &lowered.body;

        let Expr::Call { callee, args } = &body[body.root()] else {
            panic!("a call is the root of the body");
        };

        // What a step is a step of is the path written after the dot, and the path is written
        // where it is: the receiver is passed before the arguments of the call.
        assert_eq!(
            lowered.source_map.expr(*callee),
            Some(at(source, "Map::insert")),
        );
        assert_eq!(lowered.source_map.expr(args[0]), Some(at(source, "value")));
        assert_eq!(lowered.source_map.expr(args[1]), Some(at(source, "name")));
    }

    #[test]
    fn a_field_read_is_where_the_field_is_written() {
        let source = "fun main(): Int =\n    data@field\n";
        let lowered = body_of(source);
        let body = &lowered.body;

        let Expr::Field { receiver, field } = &body[body.root()] else {
            panic!("a field read is the root of the body");
        };

        assert_eq!(field.as_str(), "field");
        assert_eq!(
            lowered.source_map.expr(body.root()),
            Some(at(source, "data@field")),
        );
        assert_eq!(lowered.source_map.expr(*receiver), Some(at(source, "data")));
    }

    #[test]
    fn a_pipeline_is_the_value_it_binds_and_the_call_it_is() {
        let source = "fun main(): Int =\n    data |> consume(1, _)\n";
        let lowered = body_of(source);
        let body = &lowered.body;

        let Expr::Let {
            pat,
            expr,
            body: step,
        } = &body[body.root()]
        else {
            panic!("the `let` a pipeline lowers to is the root of the body");
        };

        // The `let` is about the spelling as a whole, the value is what stands on the left,
        // and the step is the call written on the right.
        assert_eq!(
            lowered.source_map.expr(body.root()),
            Some(at(source, "data |> consume(1, _)")),
        );
        assert_eq!(lowered.source_map.expr(*expr), Some(at(source, "data")));
        assert_eq!(
            lowered.source_map.expr(*step),
            Some(at(source, "consume(1, _)")),
        );

        // The binding is a node the lowering made up, and what it binds is the value: what
        // stands on the left is the piece of the text it reads as.
        assert_eq!(lowered.source_map.pat(*pat), Some(at(source, "data")));

        let Expr::Call { callee, args } = &body[*step] else {
            panic!("a step is a call");
        };

        assert_eq!(
            lowered.source_map.expr(*callee),
            Some(at(source, "consume")),
        );

        // Every `_` is a reference to the binding, and reads as the `_` it stands for.
        let place = *args.last().expect("the call to take arguments");
        let Expr::Path(path) = &body[place] else {
            panic!("a place is a reference to the binding");
        };

        assert_eq!(lowered.source_map.expr(place), Some(at(source, "_")));
        assert_eq!(lowered.source_map.path(*path), Some(at(source, "_")));
        assert_eq!(body[*path].anchor, PathAnchor::Binding(*pat));
    }

    #[test]
    fn a_step_written_with_the_dot_passes_its_receiver_first() {
        let source = "fun main(): Int =\n    data |> map.insert(\"k\", _)\n";
        let lowered = body_of(source);
        let body = &lowered.body;

        let Expr::Let { body: step, .. } = &body[body.root()] else {
            panic!("the `let` a pipeline lowers to is the root of the body");
        };

        assert_eq!(
            lowered.source_map.expr(*step),
            Some(at(source, "map.insert(\"k\", _)")),
        );

        let Expr::Call { callee, args } = &body[*step] else {
            panic!("a step is a call");
        };

        // The step is a call of `insert`, and the receiver is what it is passed first: `map`,
        // then `"k"`, and the value the pipeline passes last.
        assert_eq!(lowered.source_map.expr(*callee), Some(at(source, "insert")));
        assert_eq!(lowered.source_map.expr(args[0]), Some(at(source, "map")));
        assert_eq!(lowered.source_map.expr(args[1]), Some(at(source, "\"k\"")));
        assert_eq!(lowered.source_map.expr(args[2]), Some(at(source, "_")));
    }

    #[test]
    fn a_parameter_is_the_pattern_it_was_written_as() {
        let source = "fun main(value: Int): Int =\n    value\n";
        let lowered = body_of(source);
        let pat = lowered
            .body
            .params()
            .first()
            .copied()
            .expect("one parameter");

        assert!(matches!(lowered.body[pat], Pat::Bind(_)));
        assert_eq!(lowered.source_map.pat(pat), Some(at(source, "value")));
    }
}
