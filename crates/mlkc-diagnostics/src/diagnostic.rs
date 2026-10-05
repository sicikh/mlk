use mlkc_span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Lexer,
    Parser,
    /// The stage that lowers a module into the HIR: the local half of name resolution
    /// ([ADR-0005][adr-0005]).
    ///
    /// [adr-0005]: ../../docs/adr/0005-compiler-pipeline.md
    Lowering,
    Resolver,
    TypeChecker,
    Codegen,
    /// The stage that links the compiled modules of a project into a plan a host runs
    /// ([ADR-0021]).
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    Link,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Lexer => "lexer",
            Category::Parser => "parser",
            Category::Lowering => "lowering",
            Category::Resolver => "resolver",
            Category::TypeChecker => "typechecker",
            Category::Codegen => "codegen",
            Category::Link => "link",
        }
    }

    /// Two digits of the category, in the order the pipeline runs its stages, so that a
    /// reader of `error[0301]` knows which stage is talking before knowing what it says.
    pub fn as_code(&self) -> &'static str {
        match self {
            Category::Lexer => "01",
            Category::Parser => "02",
            Category::Lowering => "03",
            Category::Resolver => "04",
            Category::TypeChecker => "05",
            Category::Codegen => "06",
            Category::Link => "07",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    Error,
    Warning,
    Note,
    Help,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
            Level::Help => "help",
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Level::Error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub primary: bool,
}

impl Label {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Label {
            span,
            message: message.into(),
            primary: true,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Label {
            span,
            message: message.into(),
            primary: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: Level,
    pub category: Category,
    pub code: &'static str,
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    fn new(
        level: Level,
        category: Category,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Diagnostic {
            level,
            code,
            category,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn from_kind(kind: &impl DiagKind, msg: impl Into<String>) -> Self {
        Diagnostic::new(kind.level(), kind.category(), kind.code(), msg)
    }

    pub fn with_primary(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.labels.push(Label::primary(span, msg));
        self
    }

    pub fn with_secondary(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.labels.push(Label::secondary(span, msg));
        self
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

pub trait DiagKind {
    fn level(&self) -> Level;

    fn category(&self) -> Category;

    /// Two-digit code for the concrete error kind in the category,
    /// e.g. `01` for invalid token, `02` for unexpected token, etc.
    fn code(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use mlkc_span::{FileId, Span, TextRange, TextSize};

    use super::{Category, DiagKind, Diagnostic, Label, Level};

    /// A diagnostic kind a test controls, where the real ones come from a pass.
    struct TestKind {
        level: Level,
        category: Category,
        code: &'static str,
    }

    impl TestKind {
        fn new() -> Self {
            Self {
                level: Level::Error,
                category: Category::Parser,
                code: "01",
            }
        }
    }

    impl DiagKind for TestKind {
        fn level(&self) -> Level {
            self.level
        }

        fn category(&self) -> Category {
            self.category
        }

        fn code(&self) -> &'static str {
            self.code
        }
    }

    /// A span away from any real file, for the labels a test builds.
    fn span() -> Span {
        Span::new(
            FileId::DUMMY,
            TextRange::new(TextSize::from(0), TextSize::from(3)),
        )
    }

    #[test]
    fn from_kind_takes_its_level_category_and_code_from_the_kind() {
        let kind = TestKind {
            level: Level::Warning,
            category: Category::Resolver,
            code: "04",
        };

        let diagnostic = Diagnostic::from_kind(&kind, "an unresolved name");

        assert_eq!(diagnostic.level, Level::Warning);
        assert_eq!(diagnostic.category, Category::Resolver);
        assert_eq!(diagnostic.code, "04");
        assert_eq!(diagnostic.message, "an unresolved name");
        assert!(diagnostic.labels.is_empty());
        assert!(diagnostic.notes.is_empty());
    }

    #[test]
    fn every_level_names_itself() {
        assert_eq!(Level::Error.as_str(), "error");
        assert_eq!(Level::Warning.as_str(), "warning");
        assert_eq!(Level::Note.as_str(), "note");
        assert_eq!(Level::Help.as_str(), "help");
    }

    #[test]
    fn only_error_is_an_error() {
        assert!(Level::Error.is_error());
        assert!(!Level::Warning.is_error());
        assert!(!Level::Note.is_error());
        assert!(!Level::Help.is_error());
    }

    #[test]
    fn every_category_names_itself() {
        assert_eq!(Category::Lexer.as_str(), "lexer");
        assert_eq!(Category::Parser.as_str(), "parser");
        assert_eq!(Category::Lowering.as_str(), "lowering");
        assert_eq!(Category::Resolver.as_str(), "resolver");
        assert_eq!(Category::TypeChecker.as_str(), "typechecker");
        assert_eq!(Category::Codegen.as_str(), "codegen");
        assert_eq!(Category::Link.as_str(), "link");
    }

    #[test]
    fn a_category_codes_the_stage_in_the_order_the_pipeline_runs() {
        let categories = [
            Category::Lexer,
            Category::Parser,
            Category::Lowering,
            Category::Resolver,
            Category::TypeChecker,
            Category::Codegen,
            Category::Link,
        ];
        let codes: Vec<_> = categories.iter().map(Category::as_code).collect();

        assert_eq!(codes, ["01", "02", "03", "04", "05", "06", "07"]);
    }

    #[test]
    fn a_primary_label_is_primary_and_a_secondary_label_is_not() {
        let primary = Label::primary(span(), "the name");
        let secondary = Label::secondary(span(), String::from("the hint"));

        assert!(primary.primary);
        assert_eq!(primary.span, span());
        assert_eq!(primary.message, "the name");

        assert!(!secondary.primary);
        assert_eq!(secondary.span, span());
        assert_eq!(secondary.message, "the hint");
    }

    #[test]
    fn with_primary_appends_a_primary_label() {
        let diagnostic = Diagnostic::from_kind(&TestKind::new(), "a message")
            .with_primary(span(), "the mistake");

        assert_eq!(diagnostic.labels, [Label::primary(span(), "the mistake")]);
    }

    #[test]
    fn with_secondary_appends_a_secondary_label() {
        let diagnostic = Diagnostic::from_kind(&TestKind::new(), "a message")
            .with_secondary(span(), "the one that did it");

        assert_eq!(diagnostic.labels, [Label::secondary(
            span(),
            "the one that did it"
        )]);
    }

    #[test]
    fn with_label_appends_the_label_it_was_given() {
        let label = Label::primary(span(), "the mistake");
        let diagnostic =
            Diagnostic::from_kind(&TestKind::new(), "a message").with_label(label.clone());

        assert_eq!(diagnostic.labels, [label]);
    }

    #[test]
    fn builder_calls_keep_the_order_they_were_made_in() {
        let diagnostic = Diagnostic::from_kind(&TestKind::new(), "a message")
            .with_primary(span(), "first")
            .with_secondary(span(), "second")
            .with_note("note one")
            .with_note(String::from("note two"));

        assert_eq!(diagnostic.labels, [
            Label::primary(span(), "first"),
            Label::secondary(span(), "second"),
        ]);
        assert_eq!(diagnostic.notes, [
            "note one".to_owned(),
            "note two".to_owned()
        ]);
    }
}
