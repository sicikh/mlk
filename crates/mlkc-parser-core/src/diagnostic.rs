//! Diagnostics emitted by the parser.
//!
//! The parser is deliberately file-agnostic: it only ever sees the text it was given and
//! refers to that text with [TextRange]s. It does not know which file the text belongs to,
//! and it knows nothing about how diagnostics are classified or rendered.
//!
//! [ParseDiagnostic] therefore is a plain, self-contained value carrying exactly three
//! things:
//!
//! 1. a mandatory message and an optional [TextRange] of the offending code;
//! 2. a list of [Advice]s — extra context ("details") and suggestions ("hints"), kept in
//!    the order they were added, because that is the order they are meant to be printed in;
//! 3. the offset that has to be applied to advices added after a call to
//!    [ParseDiagnostic::set_location_offset].
//!
//! Every [ParseDiagnostic] is an error: the parser never warns.
//!
//! # Ranges, not spans
//!
//! Ranges are stored as [TextRange]s and not as [`Span`]s, even though a span is what a
//! consumer eventually needs. The parser has no file to attach to its ranges: the same
//! parser is used for whole files, for fragments, and for embedded syntax. The file is
//! attached later, at the boundary, by [ParseDiagnostic::to_diagnostic], which takes the
//! [FileId] the ranges belong to as an argument.

use std::{cmp::Ordering, fmt::Display};

use mlkc_diagnostics::{AsRange, Category, DiagKind, Diagnostic, Level};
use mlkc_rowan::{SyntaxKind, TextLen, TextRange, TextSize};
use mlkc_span::{FileId, Span};

use crate::{EOF_STR, Parser, token_source::TokenSource};

/// A diagnostic emitted by the parser.
///
/// A parse diagnostic is structured in this way:
/// 1. a mandatory message and an optional [TextRange];
/// 2. a list of [Advice]s, useful to give more information and context around the error;
/// 3. the location offset, which shifts the ranges of advices added later.
///
/// These pieces of information **are printed in this exact order**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseDiagnostic {
    /// The location where the error occurred.
    ///
    /// `None` for errors that cannot be attributed to a specific range.
    range: Option<TextRange>,
    /// The one-line summary of the error.
    pub message: String,
    /// Extra information and hints, in the order they were added.
    advices: Vec<Advice>,
    /// Offset applied to the ranges of advices added after [ParseDiagnostic::set_location_offset].
    ///
    /// The ranges stored in this struct are always absolute: the offset is applied when an
    /// advice is added, never when it is read.
    advice_offset: TextSize,
}

/// Extra information attached to a [ParseDiagnostic].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Advice {
    /// Decides how the advice is rendered.
    pub kind: AdviceKind,
    /// The message shown to the user.
    pub message: String,
    /// The range this advice points at, if any.
    pub range: Option<TextRange>,
    /// Values that were expected instead of the one that was found.
    ///
    /// Only meaningful for [AdviceKind::Hint], where they are printed as a list after the
    /// message.
    pub alternatives: Vec<String>,
}

/// The kind of an [Advice].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdviceKind {
    /// Points at another piece of code and explains how it relates to the error.
    ///
    /// Rendered as a secondary label.
    Detail,
    /// Tells the user how they could fix the error.
    ///
    /// Rendered as a note.
    Hint,
}

impl Advice {
    /// Shifts the range this advice points at.
    fn shift(&mut self, offset: TextSize) {
        if let Some(range) = self.range.as_mut() {
            *range += offset;
        }
    }
}

impl ParseDiagnostic {
    /// Creates a new diagnostic with the given message, located at `range`.
    #[must_use]
    pub fn new(message: impl Display, range: impl AsRange) -> Self {
        Self {
            range: range.as_range(),
            message: message.to_string(),
            advices: Vec::new(),
            advice_offset: TextSize::from(0),
        }
    }

    /// The location where the error occurred, if it is known.
    pub fn range(&self) -> Option<TextRange> {
        self.range
    }

    /// The advices attached to this diagnostic, in the order they were added.
    pub fn advices(&self) -> &[Advice] {
        &self.advices
    }

    /// Updates the location of this diagnostic and of its advices.
    ///
    /// The parser sees the text it is given, which is not necessarily the whole file: it can
    /// be a fragment of it, or a fragment of a synthetic document. Shifting the ranges moves
    /// the diagnostic into the coordinates of that larger document.
    ///
    /// Advices added after this call are shifted as well.
    pub fn set_location_offset(&mut self, offset: TextSize) {
        self.advice_offset += offset;

        if let Some(range) = self.range.as_mut() {
            *range += offset;
        }

        for advice in &mut self.advices {
            advice.shift(offset);
        }
    }

    /// Creates a diagnostic saying that a single node named `name` was expected at `range`.
    #[must_use]
    pub fn new_single_node(name: &str, range: TextRange, p: &impl Parser) -> Self {
        let names = format!("{} {}", article_for(name), name);
        let message = if p.source().text().text_len() <= range.start() {
            format!("Expected {names} but instead found the end of the file.")
        } else {
            format!("Expected {} but instead found '{}'.", names, p.text(range))
        };

        Self::new(message, range).with_detail(range, format!("Expected {names} here."))
    }

    /// Creates a diagnostic saying that any of the nodes named in `names` was expected at
    /// `range`.
    #[must_use]
    pub fn new_with_any(names: &[&str], range: TextRange, p: &impl Parser) -> Self {
        debug_assert!(names.len() > 1, "Requires at least 2 names");

        if names.len() < 2 {
            return Self::new_single_node(names.first().copied().unwrap_or("<missing>"), range, p);
        }

        let mut joined_names = String::new();

        for (index, name) in names.iter().enumerate() {
            if index > 0 {
                joined_names.push_str(", ");
            }

            if index == names.len() - 1 {
                joined_names.push_str("or ");
            }

            joined_names.push_str(article_for(name));
            joined_names.push(' ');
            joined_names.push_str(name);
        }

        let message = if p.source().text().text_len() <= range.start() {
            format!("Expected {joined_names} but instead found the end of the file.")
        } else {
            format!(
                "Expected {} but instead found '{}'.",
                joined_names,
                p.text(range)
            )
        };

        Self::new(message, range).with_detail(range, format!("Expected {joined_names} here."))
    }

    /// Parse diagnostics are always errors.
    pub const fn is_error(&self) -> bool {
        true
    }

    /// Attaches a detail: a message that highlights another piece of code and explains how it
    /// relates to the error.
    ///
    /// A detail is printed **after the actual error** and before the hints.
    ///
    /// ```
    /// use mlkc_parser_core::diagnostic::{AdviceKind, ParseDiagnostic};
    /// use mlkc_rowan::{TextRange, TextSize};
    ///
    /// let error_range = TextRange::new(TextSize::from(0), TextSize::from(5));
    /// let detail_range = TextRange::new(TextSize::from(6), TextSize::from(7));
    ///
    /// let diagnostic = ParseDiagnostic::new("`a` has no type", error_range)
    ///     .with_detail(detail_range, "the type is inferred from here");
    ///
    /// let advice = &diagnostic.advices()[0];
    /// assert_eq!(advice.kind, AdviceKind::Detail);
    /// assert_eq!(advice.range, Some(detail_range));
    /// ```
    #[must_use]
    pub fn with_detail(mut self, range: impl AsRange, message: impl Display) -> Self {
        self.push_advice(Advice {
            kind: AdviceKind::Detail,
            message: message.to_string(),
            range: range.as_range(),
            alternatives: Vec::new(),
        });
        self
    }

    /// Attaches a hint: a small message that suggests how the user could fix the error.
    ///
    /// Hints are rendered as the **last part** of the diagnostic.
    ///
    /// ```
    /// use mlkc_parser_core::diagnostic::{AdviceKind, ParseDiagnostic};
    /// use mlkc_rowan::{TextRange, TextSize};
    ///
    /// let range = TextRange::new(TextSize::from(0), TextSize::from(5));
    ///
    /// let diagnostic =
    ///     ParseDiagnostic::new("this is wrong!", range).with_hint("You should delete the code");
    ///
    /// let advice = &diagnostic.advices()[0];
    /// assert_eq!(advice.kind, AdviceKind::Hint);
    /// assert_eq!(advice.message, "You should delete the code");
    /// ```
    #[must_use]
    pub fn with_hint(mut self, message: impl Display) -> Self {
        self.push_advice(Advice {
            kind: AdviceKind::Hint,
            message: message.to_string(),
            range: None,
            alternatives: Vec::new(),
        });
        self
    }

    /// Attaches a hint that lists the values or characters that were expected.
    ///
    /// ```
    /// use mlkc_parser_core::diagnostic::ParseDiagnostic;
    /// use mlkc_rowan::{TextRange, TextSize};
    ///
    /// let range = TextRange::new(TextSize::from(0), TextSize::from(5));
    ///
    /// let diagnostic = ParseDiagnostic::new("this is wrong!", range)
    ///     .with_alternatives("Expected one of the following values:", &["foo", "bar"]);
    ///
    /// let advice = &diagnostic.advices()[0];
    /// assert_eq!(advice.alternatives, ["foo", "bar"]);
    /// ```
    #[must_use]
    pub fn with_alternatives(
        mut self,
        message: impl Display,
        alternatives: &[impl Display],
    ) -> Self {
        self.push_advice(Advice {
            kind: AdviceKind::Hint,
            message: message.to_string(),
            range: None,
            alternatives: alternatives.iter().map(ToString::to_string).collect(),
        });
        self
    }

    /// Retrieves the range that belongs to the diagnostic.
    pub(crate) fn diagnostic_range(&self) -> Option<TextRange> {
        self.range
    }

    /// Adds an advice, shifting it by the location offset accumulated so far.
    fn push_advice(&mut self, mut advice: Advice) {
        advice.shift(self.advice_offset);
        self.advices.push(advice);
    }

    /// Converts this parser diagnostic into the compiler-wide [Diagnostic].
    ///
    /// The parser only knows the ranges of the text it was given, so the file those ranges
    /// belong to has to be supplied by the caller: the driver knows the [FileId] of the file
    /// from the VFS (or from the parse unit).
    ///
    /// The conversion is:
    ///
    /// - the [message](ParseDiagnostic::message) becomes the diagnostic message;
    /// - the primary range becomes a primary label;
    /// - [AdviceKind::Detail] advices become secondary labels;
    /// - [AdviceKind::Hint] advices become notes, with their alternatives printed as a list.
    ///
    /// A [Diagnostic] keeps labels and notes in separate lists, so the relative order of the
    /// advices survives within each list, but not between them.
    #[must_use]
    pub fn to_diagnostic(&self, file: FileId) -> Diagnostic {
        let mut diagnostic = Diagnostic::from_kind(self, self.message.clone());

        if let Some(range) = self.range {
            diagnostic = diagnostic.with_primary(Span::new(file, range), "");
        }

        for advice in &self.advices {
            diagnostic = match advice.kind {
                AdviceKind::Detail => {
                    match advice.range {
                        Some(range) => {
                            diagnostic
                                .with_secondary(Span::new(file, range), advice.message.clone())
                        },
                        // A detail without a range has nothing to point at, so it is kept as a
                        // note instead of being dropped.
                        None => diagnostic.with_note(advice.message.clone()),
                    }
                },
                AdviceKind::Hint => {
                    let mut note = advice.message.clone();

                    for alternative in &advice.alternatives {
                        note.push_str("\n- ");
                        note.push_str(alternative);
                    }

                    diagnostic.with_note(note)
                },
            };
        }

        diagnostic
    }
}

impl DiagKind for ParseDiagnostic {
    fn level(&self) -> Level {
        Level::Error
    }

    fn category(&self) -> Category {
        Category::Parser
    }

    /// Parse diagnostics carry no code of their own: they all share the code of the
    /// [Category::Parser] category.
    fn code(&self) -> &'static str {
        Category::Parser.as_code()
    }
}

/// Converts a value produced by a parse rule into a [ParseDiagnostic].
///
/// Some errors can only be materialized once the parser has been consulted (which token was
/// found, where the file ends, etc.), so a parse rule returns one of these values and the
/// [Parser] converts it when the error is reported.
pub trait ToDiagnostic<P>
where
    P: Parser,
{
    fn into_diagnostic(self, p: &P) -> ParseDiagnostic;
}

impl<P: Parser> ToDiagnostic<P> for ParseDiagnostic {
    fn into_diagnostic(self, _: &P) -> ParseDiagnostic {
        self
    }
}

#[must_use]
pub fn expected_token<K>(token: K) -> ExpectedToken
where
    K: SyntaxKind,
{
    ExpectedToken(
        token
            .to_string()
            .expect("Expected token to be a punctuation or keyword."),
    )
}

#[must_use]
pub fn expected_token_any<K: SyntaxKind>(tokens: &[K]) -> ExpectedTokens {
    use std::fmt::Write;
    let mut expected = String::new();

    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            expected.push_str(", ");
        }

        if index == tokens.len() - 1 {
            expected.push_str("or ");
        }

        let _ = write!(
            &mut expected,
            "'{}'",
            token
                .to_string()
                .expect("Expected token to be a punctuation or keyword.")
        );
    }

    ExpectedTokens(expected)
}

pub struct ExpectedToken(&'static str);

impl<P> ToDiagnostic<P> for ExpectedToken
where
    P: Parser,
{
    fn into_diagnostic(self, p: &P) -> ParseDiagnostic {
        if p.cur() == P::Kind::EOF {
            p.err_builder(
                format!("expected `{}` but instead the file ends", self.0),
                p.cur_range(),
            )
            .with_detail(p.cur_range(), "the file ends here")
        } else if self.0 == EOF_STR {
            p.err_builder(
                format!(
                    "expected the file to end but instead found an excess `{}`",
                    p.cur_text()
                ),
                p.cur_range(),
            )
            .with_hint(format!("Remove {}", p.cur_text()))
        } else {
            p.err_builder(
                format!("expected `{}` but instead found `{}`", self.0, p.cur_text()),
                p.cur_range(),
            )
            .with_hint(format!("Remove {}", p.cur_text()))
        }
    }
}

pub struct ExpectedTokens(String);

impl<P> ToDiagnostic<P> for ExpectedTokens
where
    P: Parser,
{
    fn into_diagnostic(self, p: &P) -> ParseDiagnostic {
        if p.cur() == P::Kind::EOF {
            p.err_builder(
                format!("expected {} but instead the file ends", self.0),
                p.cur_range(),
            )
            .with_detail(p.cur_range(), "the file ends here")
        } else {
            p.err_builder(
                format!("expected {} but instead found `{}`", self.0, p.cur_text()),
                p.cur_range(),
            )
            .with_hint(format!("Remove {}", p.cur_text()))
        }
    }
}

/// Creates a diagnostic saying that the node `name` was expected at range
pub fn expected_node(name: &str, range: TextRange, p: &impl Parser) -> ParseDiagnostic {
    ParseDiagnostic::new_single_node(name, range, p)
}

/// Creates a diagnostic saying that any of the nodes in `names` was expected at range
pub fn expected_any(names: &[&str], range: TextRange, p: &impl Parser) -> ParseDiagnostic {
    ParseDiagnostic::new_with_any(names, range, p)
}

/// Creates a diagnostic with message "Unexpected value." and then it lists the values that should be expected.
pub fn expect_one_of(names: &[&str], range: TextRange) -> ParseDiagnostic {
    ParseDiagnostic::new("Unexpected value or character.", range)
        .with_alternatives("Expected one of:", names)
}

fn article_for(name: &str) -> &'static str {
    match name.bytes().next() {
        Some(b'a' | b'e' | b'i' | b'o' | b'u') => "an",
        _ => "a",
    }
}

/// Merges two lists of parser diagnostics. Only keeps the error from the first collection if two start at the same range.
///
/// The two lists must be so sorted by their source range in increasing order.
pub fn merge_diagnostics(
    first: Vec<ParseDiagnostic>,
    second: Vec<ParseDiagnostic>,
) -> Vec<ParseDiagnostic> {
    if first.is_empty() {
        return second;
    }

    if second.is_empty() {
        return first;
    }

    let mut merged = Vec::new();

    let mut first_iter = first.into_iter();
    let mut second_iter = second.into_iter();

    let mut current_first: Option<ParseDiagnostic> = first_iter.next();
    let mut current_second: Option<ParseDiagnostic> = second_iter.next();

    loop {
        match (current_first, current_second) {
            (Some(first_item), Some(second_item)) => {
                let (first, second) = match (
                    first_item.diagnostic_range(),
                    second_item.diagnostic_range(),
                ) {
                    (Some(first_range), Some(second_range)) => {
                        match first_range.start().cmp(&second_range.start()) {
                            Ordering::Less => {
                                merged.push(first_item);
                                (first_iter.next(), Some(second_item))
                            },
                            Ordering::Equal => {
                                // Only keep one error, skip the one from the second list.
                                (Some(first_item), second_iter.next())
                            },
                            Ordering::Greater => {
                                merged.push(second_item);
                                (Some(first_item), second_iter.next())
                            },
                        }
                    },
                    (Some(_), None) => {
                        merged.push(second_item);
                        (Some(first_item), second_iter.next())
                    },
                    (None, Some(_)) => {
                        merged.push(first_item);
                        (first_iter.next(), Some(second_item))
                    },
                    (None, None) => {
                        merged.push(first_item);
                        merged.push(second_item);

                        (first_iter.next(), second_iter.next())
                    },
                };

                current_first = first;
                current_second = second;
            },

            (None, None) => return merged,
            (Some(first_item), None) => {
                merged.push(first_item);
                merged.extend(first_iter);
                return merged;
            },
            (None, Some(second_item)) => {
                merged.push(second_item);
                merged.extend(second_iter);
                return merged;
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use mlkc_syntax::SyntaxKind::{self, *};

    use super::*;
    use crate::{ParserContext, token_source::Trivia};

    const FILE: FileId = FileId::from_raw(0);

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(TextSize::from(start), TextSize::from(end))
    }

    /// A token source that keeps reporting the same token until it is bumped.
    struct TestTokenSource {
        text: &'static str,
        kind: SyntaxKind,
        range: TextRange,
    }

    impl TestTokenSource {
        /// A source positioned at `kind`, which spans `range`.
        fn at(text: &'static str, kind: SyntaxKind, range: TextRange) -> Self {
            Self { text, kind, range }
        }

        /// A source positioned at the end of `text`.
        fn end_of(text: &'static str) -> Self {
            let end = TextSize::try_from(text.len()).expect("text is too long");
            Self {
                text,
                kind: EOF,
                range: TextRange::empty(end),
            }
        }
    }

    impl TokenSource for TestTokenSource {
        type Kind = SyntaxKind;

        fn current(&self) -> Self::Kind {
            self.kind
        }

        fn current_range(&self) -> TextRange {
            self.range
        }

        fn text(&self) -> &str {
            self.text
        }

        fn has_preceding_line_break(&self) -> bool {
            false
        }

        fn bump(&mut self) {
            *self = Self::end_of(self.text);
        }

        fn skip_as_trivia(&mut self) {
            self.bump();
        }

        fn finish(self) -> (Vec<Trivia>, Vec<ParseDiagnostic>) {
            (Vec::new(), Vec::new())
        }
    }

    struct TestParser {
        context: ParserContext<SyntaxKind>,
        source: TestTokenSource,
    }

    impl TestParser {
        fn new(source: TestTokenSource) -> Self {
            Self {
                context: ParserContext::new(),
                source,
            }
        }
    }

    impl Parser for TestParser {
        type Kind = SyntaxKind;
        type Source = TestTokenSource;

        fn context(&self) -> &ParserContext<Self::Kind> {
            &self.context
        }

        fn context_mut(&mut self) -> &mut ParserContext<Self::Kind> {
            &mut self.context
        }

        fn source(&self) -> &Self::Source {
            &self.source
        }

        fn source_mut(&mut self) -> &mut Self::Source {
            &mut self.source
        }
    }

    #[test]
    fn new_diagnostic_has_a_message_and_a_range() {
        let diagnostic = ParseDiagnostic::new("something is wrong", range(4, 8));

        assert_eq!(diagnostic.message, "something is wrong");
        assert_eq!(diagnostic.range(), Some(range(4, 8)));
        assert_eq!(diagnostic.diagnostic_range(), Some(range(4, 8)));
        assert!(diagnostic.advices().is_empty());
        assert!(diagnostic.is_error());
    }

    #[test]
    fn new_diagnostic_without_a_range() {
        let diagnostic = ParseDiagnostic::new("something is wrong", None::<TextRange>);

        assert_eq!(diagnostic.range(), None);
        assert!(diagnostic.to_diagnostic(FILE).labels.is_empty());
    }

    #[test]
    fn advices_keep_the_order_and_the_shape_they_were_added_with() {
        let diagnostic = ParseDiagnostic::new("something is wrong", range(0, 1))
            .with_detail(range(10, 12), "the detail")
            .with_hint("the hint")
            .with_alternatives("expected one of:", &["`a`", "`b`"]);

        let advices = diagnostic.advices();
        assert_eq!(advices.len(), 3);

        assert_eq!(advices[0].kind, AdviceKind::Detail);
        assert_eq!(advices[0].message, "the detail");
        assert_eq!(advices[0].range, Some(range(10, 12)));
        assert!(advices[0].alternatives.is_empty());

        assert_eq!(advices[1].kind, AdviceKind::Hint);
        assert_eq!(advices[1].message, "the hint");
        assert_eq!(advices[1].range, None);

        assert_eq!(advices[2].kind, AdviceKind::Hint);
        assert_eq!(advices[2].message, "expected one of:");
        assert_eq!(advices[2].alternatives, ["`a`", "`b`"]);
    }

    #[test]
    fn location_offset_shifts_the_primary_range_and_the_advices() {
        let mut diagnostic = ParseDiagnostic::new("something is wrong", range(0, 3))
            .with_detail(range(3, 5), "here");

        diagnostic.set_location_offset(TextSize::from(100));
        // Advices added after the offset are shifted as well.
        let diagnostic = diagnostic.with_hint("fix it");

        assert_eq!(diagnostic.range(), Some(range(100, 103)));
        assert_eq!(diagnostic.advices()[0].range, Some(range(103, 105)));
        assert_eq!(diagnostic.advices()[1].range, None);

        // Offsets accumulate.
        let mut diagnostic = diagnostic;
        diagnostic.set_location_offset(TextSize::from(10));
        assert_eq!(diagnostic.range(), Some(range(110, 113)));
    }

    #[test]
    fn converts_into_the_compiler_diagnostic() {
        let diagnostic = ParseDiagnostic::new("expected `}`", range(10, 11))
            .with_detail(range(0, 1), "the block starts here")
            .with_hint("add a closing brace")
            .with_alternatives("expected one of:", &["`}`", "`;`"]);

        let converted = diagnostic.to_diagnostic(FILE);

        assert_eq!(converted.level, Level::Error);
        assert_eq!(converted.category, Category::Parser);
        assert_eq!(converted.message, "expected `}`");

        assert_eq!(converted.labels.len(), 2);
        assert_eq!(converted.labels[0].span, Span::new(FILE, range(10, 11)));
        assert_eq!(converted.labels[0].message, "");
        assert!(converted.labels[0].primary);
        assert_eq!(converted.labels[1].span, Span::new(FILE, range(0, 1)));
        assert_eq!(converted.labels[1].message, "the block starts here");
        assert!(!converted.labels[1].primary);

        assert_eq!(converted.notes, [
            "add a closing brace".to_string(),
            "expected one of:\n- `}`\n- `;`".to_string(),
        ]);
    }

    #[test]
    fn reports_the_end_of_the_file() {
        let p = TestParser::new(TestTokenSource::end_of("let x = 1"));

        let diagnostic = ParseDiagnostic::new_single_node("expression", range(9, 9), &p);
        assert_eq!(
            diagnostic.message,
            "Expected an expression but instead found the end of the file."
        );
        assert_eq!(
            diagnostic.advices()[0].message,
            "Expected an expression here."
        );

        let diagnostic = expected_token(L_PAREN).into_diagnostic(&p);
        assert_eq!(diagnostic.message, "expected `(` but instead the file ends");
        assert_eq!(diagnostic.advices()[0].message, "the file ends here");
    }

    #[test]
    fn reports_the_found_token() {
        let p = TestParser::new(TestTokenSource::at("let x = 1", IDENT, range(0, 3)));

        let diagnostic = ParseDiagnostic::new_single_node("expression", range(0, 3), &p);
        assert_eq!(
            diagnostic.message,
            "Expected an expression but instead found 'let'."
        );

        let diagnostic = expected_token(L_PAREN).into_diagnostic(&p);
        assert_eq!(diagnostic.message, "expected `(` but instead found `let`");
        assert_eq!(diagnostic.advices()[0].message, "Remove let");
    }

    #[test]
    fn merges_diagnostics_starting_at_the_same_range() {
        let first = vec![
            ParseDiagnostic::new("first", range(0, 1)),
            ParseDiagnostic::new("second", range(4, 5)),
        ];
        let second = vec![
            ParseDiagnostic::new("duplicate", range(0, 2)),
            ParseDiagnostic::new("third", range(7, 8)),
        ];

        let merged = merge_diagnostics(first, second);
        let messages: Vec<&str> = merged.iter().map(|it| it.message.as_str()).collect();

        assert_eq!(messages, ["first", "second", "third"]);
    }

    #[test]
    fn merges_diagnostics_without_a_range() {
        let first = vec![ParseDiagnostic::new("first", None::<TextRange>)];
        let second = vec![ParseDiagnostic::new("second", range(0, 1))];

        let merged = merge_diagnostics(first, second);
        let messages: Vec<&str> = merged.iter().map(|it| it.message.as_str()).collect();

        assert_eq!(messages, ["first", "second"]);
    }
}
