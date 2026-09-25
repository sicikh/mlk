#![cfg(test)]
#![expect(unused_mut)]

use std::{sync::mpsc::channel, thread, time::Duration};

use mlkc_parser_core::lexer::{BufferedLexer, Lexer as LexerTrait};
use mlkc_syntax::{
    SyntaxKind::{self, *},
    TextRange, TextSize,
};
use quickcheck_macros::quickcheck;

use super::Lexer;

/// Asserts the result of lexing a piece of source code: the kinds and the lengths
/// of the tokens, that there are no other tokens, and that the source can be
/// reconstructed from the tokens alone (the lexer is lossless).
macro_rules! assert_lex {
    ($src:expr, $($kind:ident:$len:expr $(,)?)*) => {{
        let source = $src;
        let mut lexer = Lexer::from_str(source);
        let mut tokens = Vec::new();

        loop {
            let kind = LexerTrait::next_token(&mut lexer, ());

            if kind == EOF {
                break;
            }

            tokens.push((lexer.current(), lexer.current_range()));
        }

        let mut index = 0;
        let mut reconstructed = String::with_capacity(source.len());

        $(
            assert!(
                index < tokens.len(),
                "expected more tokens: token {} is {:?}",
                index,
                SyntaxKind::$kind,
            );

            let (actual_kind, actual_range) = tokens[index];

            assert_eq!(
                actual_kind,
                SyntaxKind::$kind,
                "expected token {} to be {:?}, but it is {:?}",
                index,
                SyntaxKind::$kind,
                actual_kind,
            );

            assert_eq!(
                actual_range.len(),
                TextSize::from($len),
                "expected token {} to be {} bytes long, but it is {:?}",
                index,
                $len,
                actual_range.len(),
            );

            reconstructed.push_str(&source[actual_range]);
            index += 1;
        )*

        assert_eq!(
            index,
            tokens.len(),
            "expected {} tokens, but the lexer returned {}: {:?}",
            index,
            tokens.len(),
            &tokens[index..],
        );

        assert_eq!(source, reconstructed, "the tokens do not reconstruct the source");
    }};
}

// This is for testing if the lexer is truly lossless: it lexes random strings and
// puts them back together from the tokens it produced.
#[quickcheck]
fn losslessness(string: String) -> bool {
    // The lexer runs on another thread, so that a lexer that does not advance
    // does not hang the test forever.
    let source = string.clone();
    let (sender, receiver) = channel();

    thread::spawn(move || {
        let mut lexer = Lexer::from_str(&source);
        let mut ranges = Vec::new();

        while let Some(token) = lexer.next_token() {
            ranges.push(token.range());
        }

        sender.send(ranges).expect("could not send the tokens");
    });

    let ranges = receiver
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_else(|_| panic!("the lexer does not advance on {string:?}"));

    let mut reconstructed = String::with_capacity(string.len());

    for range in ranges {
        reconstructed.push_str(&string[range]);
    }

    string == reconstructed
}

#[test]
fn empty() {
    assert_lex! {
        "",
    }
}

#[test]
fn identifier() {
    assert_lex! {
        "Abcdefg",
        IDENT:7
    }

    assert_lex! {
        "println-int",
        IDENT:11
    }

    assert_lex! {
        "x - 1",
        IDENT:1,
        WHITESPACE:1,
        MINUS:1,
        WHITESPACE:1,
        INT_LITERAL:1
    }
}

#[test]
fn dash_belongs_to_the_identifier_between_identifier_characters() {
    assert_lex! {
        "x-1",
        IDENT:3
    }

    assert_lex! {
        "foo-",
        IDENT:3,
        MINUS:1
    }

    assert_lex! {
        "-_x",
        MINUS:1,
        IDENT:2
    }

    // A number, on the other hand, is never the start of an identifier.
    assert_lex! {
        "1-2",
        INT_LITERAL:1,
        MINUS:1,
        INT_LITERAL:1
    }
}

#[test]
fn underscore_is_not_an_identifier() {
    assert_lex! {
        "_",
        UNDERSCORE:1
    }

    assert_lex! {
        "_x",
        IDENT:2
    }

    assert_lex! {
        "_x-y_2",
        IDENT:6
    }
}

#[test]
fn keywords() {
    let keywords = ["fun", "in", "let", "module", "pub", "type"];

    for keyword in keywords {
        let kind = SyntaxKind::from_keyword(keyword)
            .expect("expected `SyntaxKind::from_keyword` to know the keyword");

        let mut lexer = Lexer::from_str(keyword);
        LexerTrait::next_token(&mut lexer, ());

        assert_eq!(lexer.current(), kind, "expected `{keyword}` to be {kind:?}");
        assert_eq!(
            lexer.current_range().len(),
            TextSize::from(keyword.len() as u32),
            "expected `{keyword}` to be lexed as a whole"
        );
        assert_eq!(LexerTrait::next_token(&mut lexer, ()), EOF);
    }
}

#[test]
fn keywords_are_not_identifiers() {
    assert_lex! {
        "funny fun-fun",
        IDENT:5,
        WHITESPACE:1,
        IDENT:7
    }

    assert_lex! {
        "module-like letting inner",
        IDENT:11,
        WHITESPACE:1,
        IDENT:7,
        WHITESPACE:1,
        IDENT:5
    }
}

#[test]
fn punctuators() {
    assert_lex! {
        "@(),:.{}[];=",
        AT:1,
        L_PAREN:1,
        R_PAREN:1,
        COMMA:1,
        COLON:1,
        DOT:1,
        L_CURLY:1,
        R_CURLY:1,
        L_BRACK:1,
        R_BRACK:1,
        SEMICOLON:1,
        EQ:1
    }

    assert_lex! {
        "->",
        ARROW:2
    }
}

#[test]
fn operators() {
    assert_lex! {
        "+*-/",
        PLUS:1,
        STAR:1,
        MINUS:1,
        SLASH:1
    }

    assert_lex! {
        "==!=<><=>=&&||",
        EQ2:2,
        BANG_EQ:2,
        LT:1,
        GT:1,
        LT_EQ:2,
        GT_EQ:2,
        AND2:2,
        OR2:2
    }
}

#[test]
fn numbers() {
    assert_lex! {
        "0 42 1024",
        INT_LITERAL:1,
        WHITESPACE:1,
        INT_LITERAL:2,
        WHITESPACE:1,
        INT_LITERAL:4
    }

    assert_lex! {
        "007",
        INT_LITERAL:3
    }

    assert_lex! {
        "1a",
        INT_LITERAL:1,
        IDENT:1
    }
}

#[test]
fn strings() {
    assert_lex! {
        r#""""#,
        STRING_LITERAL:2
    }

    assert_lex! {
        r#""mlk""#,
        STRING_LITERAL:5
    }

    assert_lex! {
        r#""a b""#,
        STRING_LITERAL:5
    }

    // There are no escapes yet: the string ends at the first quote.
    assert_lex! {
        r#""let""#,
        STRING_LITERAL:5
    }

    assert_lex! {
        r#""abc""def""#,
        STRING_LITERAL:5,
        STRING_LITERAL:5
    }
}

#[test]
fn unterminated_string() {
    // One broken token for the whole run, not one for every character.
    assert_lex! {
        r#""abc"#,
        ERROR_TOKEN:4
    }

    assert_lex! {
        "\"abc\nlet",
        ERROR_TOKEN:4,
        NEWLINE:1,
        LET_KW:3
    }
}

#[test]
fn comments() {
    assert_lex! {
        "//",
        COMMENT:2
    }

    assert_lex! {
        "// a comment\n",
        COMMENT:12,
        NEWLINE:1
    }

    assert_lex! {
        "/* */",
        MULTILINE_COMMENT:5
    }

    assert_lex! {
        "/* a\nb */",
        MULTILINE_COMMENT:9
    }
}

#[test]
fn all_whitespace() {
    assert_lex! {
        "\t\t",
        WHITESPACE:2
    }

    assert_lex! {
        "\n\t\t",
        NEWLINE:1,
        WHITESPACE:2
    }

    assert_lex! {
        "\r\n\t\t",
        NEWLINE:2,
        WHITESPACE:2
    }

    assert_lex! {
        "\n\n",
        NEWLINE:1,
        NEWLINE:1
    }

    assert_lex! {
        "\r\n\r\n",
        NEWLINE:2,
        NEWLINE:2
    }

    assert_lex! {
        "\r\r\r\r",
        NEWLINE:1,
        NEWLINE:1,
        NEWLINE:1,
        NEWLINE:1
    }
}

#[test]
fn newline_and_whitespace_are_two_tokens() {
    assert_lex! {
        "\n ",
        NEWLINE:1,
        WHITESPACE:1
    }

    assert_lex! {
        " \n",
        WHITESPACE:1,
        NEWLINE:1
    }

    assert_lex! {
        " \r\n ",
        WHITESPACE:1,
        NEWLINE:2,
        WHITESPACE:1
    }

    assert_lex! {
        "let a\n = 1",
        LET_KW:3,
        WHITESPACE:1,
        IDENT:1,
        NEWLINE:1,
        WHITESPACE:1,
        EQ:1,
        WHITESPACE:1,
        INT_LITERAL:1
    }
}

#[test]
fn errors() {
    assert_lex! {
        "$",
        ERROR_TOKEN:1
    }

    assert_lex! {
        "a$b",
        IDENT:1,
        ERROR_TOKEN:1,
        IDENT:1
    }

    assert_lex! {
        "let $",
        LET_KW:3,
        WHITESPACE:1,
        ERROR_TOKEN:1
    }

    assert_lex! {
        "!",
        ERROR_TOKEN:1
    }

    assert_lex! {
        "&&&",
        AND2:2,
        ERROR_TOKEN:1
    }

    // Identifiers are ASCII for now: a letter of another alphabet is not a name.
    assert_lex! {
        "αβ",
        ERROR_TOKEN:2,
        ERROR_TOKEN:2
    }
}

#[test]
fn byte_order_mark() {
    assert_lex! {
        "\u{feff}fun",
        UNICODE_BOM:3,
        FUN_KW:3
    }
}

#[test]
fn without_lookahead() {
    let mut buffered = BufferedLexer::new(Lexer::from_str("let a\n = 5"));

    assert_eq!(buffered.next_token(()), LET_KW);
    assert_eq!(buffered.current(), LET_KW);
    assert!(!buffered.has_preceding_line_break());
    assert_eq!(
        buffered.current_range(),
        TextRange::at(TextSize::from(0), TextSize::from(3))
    );

    assert_eq!(buffered.next_token(()), WHITESPACE);
    assert_eq!(buffered.next_token(()), IDENT);
    assert_eq!(buffered.next_token(()), NEWLINE);
    assert_eq!(buffered.next_token(()), WHITESPACE);
    assert_eq!(buffered.next_token(()), EQ);
    assert!(buffered.has_preceding_line_break());
    assert_eq!(buffered.next_token(()), WHITESPACE);
    assert_eq!(buffered.next_token(()), INT_LITERAL);
    assert_eq!(buffered.next_token(()), EOF);
}

#[test]
fn lookahead() {
    let mut buffered = BufferedLexer::new(Lexer::from_str("let a\n = 5"));

    buffered.next_token(());
    assert_eq!(buffered.current(), LET_KW);

    {
        let lookahead: Vec<SyntaxKind> = buffered
            .lookahead_iter()
            .map(|lookahead| lookahead.kind())
            .collect();

        assert_eq!(lookahead, [
            WHITESPACE,
            IDENT,
            NEWLINE,
            WHITESPACE,
            EQ,
            WHITESPACE,
            INT_LITERAL,
            EOF
        ]);
    }

    // Looking ahead does not move the token source.
    assert_eq!(buffered.current(), LET_KW);
    assert_eq!(buffered.next_token(()), WHITESPACE);

    {
        let mut lookahead = buffered.lookahead_iter();

        let first = lookahead.next().expect("there is a token after the space");
        let second = lookahead.next().expect("there is a token after the name");
        let third = lookahead
            .next()
            .expect("there is a token after the line break");
        let fourth = lookahead.next().expect("there is the `=`");

        assert_eq!(first.kind(), IDENT);
        assert_eq!(second.kind(), NEWLINE);
        assert_eq!(third.kind(), WHITESPACE);
        assert_eq!(fourth.kind(), EQ);
        assert!(fourth.has_preceding_line_break());
    }

    assert_eq!(buffered.next_token(()), IDENT);
    assert_eq!(buffered.next_token(()), NEWLINE);
    assert_eq!(buffered.next_token(()), WHITESPACE);
    assert_eq!(buffered.next_token(()), EQ);
    assert!(buffered.has_preceding_line_break());
    assert_eq!(buffered.next_token(()), WHITESPACE);
    assert_eq!(buffered.next_token(()), INT_LITERAL);
    assert_eq!(buffered.next_token(()), EOF);
}
