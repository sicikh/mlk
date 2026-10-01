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
