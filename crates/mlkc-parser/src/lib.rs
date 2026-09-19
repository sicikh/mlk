mod diagnostic;
mod event;
mod lexer;
mod tree_sink;

pub use diagnostic::*;
use event::*;
use lexer::*;
use mlkc_rowan::AstNode;
use mlkc_syntax::*;

use crate::tree_sink::LosslessTreeSink;

pub fn parse_module(source: &str) -> (RootModule, Vec<ParseDiagnostic>) {
    let (tokens, trivia) = lex(source);
    let (events, diags) = Parser::new(tokens).parse();
    let mut sink = LosslessTreeSink::<'_, _, MlkSyntaxFactory>::new(source, trivia.as_slice());
    process(&mut sink, events, diags);
    let (root, diags) = sink.finish();
    (RootModule::cast(root).unwrap(), diags)
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
    events: Vec<Event>,
    diags: Vec<ParseDiagnostic>,
}

enum SExpRes {
    Ok,
    Eof,
    RParen,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
            events: Vec::new(),
            diags: Vec::new(),
        }
    }

    fn parse(mut self) -> (Vec<Event>, Vec<ParseDiagnostic>) {
        self.start(ROOT_MODULE);
        self.expr_list();
        self.end();
        (self.events, self.diags)
    }

    fn sexp(&mut self) -> SExpRes {
        let t = match self.current_kind() {
            None => return SExpRes::Eof,
            Some(kind) if kind.is_right_paren() => return SExpRes::RParen,
            Some(t) => t,
        };
        match t {
            t if t.is_left_paren() => {
                self.start(LIST_EXPR);
                self.bump();
                self.expr_list();
                // TODO: expect the same paren
                self.bump();
                self.end();
            },
            NUMBER_LITERAL | BOOLEAN_LITERAL | STRING_LITERAL | IDENT => {
                let atom_kind = match t {
                    NUMBER_LITERAL => NUMBER_LITERAL_ATOM,
                    STRING_LITERAL => STRING_LITERAL_ATOM,
                    BOOLEAN_LITERAL => BOOLEAN_LITERAL_ATOM,
                    IDENT => SYMBOL_ATOM,
                    _ => unreachable!(),
                };

                self.start(atom_kind);
                self.bump();
                self.end();
            },
            BOGUS_EXPR => {
                self.start(BOGUS_EXPR);
                self.bump();
                self.end();
            },
            _ => unreachable!(),
        }

        SExpRes::Ok
    }

    fn expr_list(&mut self) {
        self.start(EXPR_LIST);
        loop {
            match self.sexp() {
                SExpRes::Ok => (),
                SExpRes::Eof => break,
                // TODO: receive flag to error on RParen
                SExpRes::RParen => break,
            }
        }
        self.end()
    }

    fn current_kind(&self) -> Option<MlkSyntaxKind> {
        if self.current < self.tokens.len() {
            Some(self.tokens[self.current].kind)
        } else {
            None
        }
    }

    fn bump(&mut self) {
        let token = &self.tokens[self.current];
        self.events.push(Event::Token {
            kind: token.kind,
            end: token.range.end(),
        });
        self.current += 1;
    }

    fn start(&mut self, kind: MlkSyntaxKind) {
        self.events.push(Event::Start {
            kind,
            forward_parent: None,
        });
    }
    fn end(&mut self) {
        self.events.push(Event::Finish);
    }

    fn error(&mut self, _msg: String) {
        // self.diags.push(diag(msg, DiagnosticCategory::Parse));
    }
}

#[cfg(test)]
mod tests {
    use mlkc_rowan::AstNodeList;

    use super::*;

    #[test]
    fn test_parse_module() {
        let source = "(define (square x) (* x x))";
        let (root, _diags) = parse_module(source);
        let text = root
            .items()
            .first()
            .unwrap()
            .as_list()
            .unwrap()
            .inner()
            .iter()
            .nth(1)
            .unwrap()
            .to_trimmed_string();

        assert_eq!(text, "(square x)");
    }
}
