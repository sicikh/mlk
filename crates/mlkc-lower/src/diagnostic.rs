//! What lowering reports: the errors of a module, and where they are.
//!
//! An error is a value, not a sentence. What lowering found is a kind --- the module declares a
//! name twice, an attribute means nothing, an external function has a body --- and the facts it
//! is about, in the terms of the HIR: a name, a path, the declaration that lost. A caller
//! matches on it, counts it, and gives it a code ([`DiagKind`]); a reader reads its
//! [message](LoweringError::message), which is a rendering of the facts and a step of its own.

use std::fmt;

use mlkc_diagnostics::{Category, DiagKind, Diagnostic, Level};
use mlkc_hir_def::{ClassLoc, EntityLoc, FunctionLoc, ItemLoc, ItemLocLike, Name, PlainPath};
use mlkc_span::Span;

/// What a module says that the HIR cannot hold, and what the language does not allow.
///
/// The facts of an error are the ones the HIR knows: a name, a path of names, the declaration
/// that is out of place. Nothing here is a position or a text range: where the error is, is
/// [`LoweringDiag::span`], and the rest of the HIR is what a rendering is made of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoweringError {
    /// The module declares one name twice where the name means one thing.
    DuplicateName {
        /// The declaration that lost: the first declaration is what the name denotes.
        item: ItemLoc,
    },
    /// A declaration carries an attribute the language has no meaning for.
    UnknownAttribute {
        /// The name written after the `@`.
        name: Name,
    },
    /// A declaration writes the same attribute twice.
    ///
    /// An attribute is a thing the declaration says about itself, and a declaration that
    /// says it twice says no more than one that says it once: the second writing is what a
    /// reader is told about.
    RepeatedAttribute {
        /// The name written after the `@`.
        name: Name,
    },
    /// A function declares one parameter name twice.
    ///
    /// The parameters of a function are what its body binds, and a name a body reads is one
    /// name: two parameters written under one name are two arguments a reader cannot tell
    /// apart, and which of the two a use of the name means is not something the module says.
    DuplicateParameterName {
        /// The function.
        function: FunctionLoc,
        /// The name the parameters share.
        name: Name,
    },
    /// A string literal holds an escape the language has no meaning for.
    ///
    /// What an escape the language does know is a character it stands for, and one it does
    /// not know is the two characters it is written with: a literal that holds one is read as
    /// far as it can be read, and what a reader is told about is the sequence.
    UnknownEscape {
        /// The escape as it is written: the backslash, and the character it escapes.
        escape: String,
    },
    /// An integer literal does not fit the value the HIR holds it in.
    IntegerLiteralTooLarge {
        /// The literal as the module wrote it.
        literal: String,
    },
    /// A path that names something of the project carries type arguments.
    ///
    /// A path of the project is a path of names: what a segment could be applied to is a type
    /// of the language, and the stage that resolves a path is what reads the names after it.
    PathIsNotPlain {
        /// The path as far as it is a path of names.
        path: PlainPath,
    },
    /// `@extern` is on a function that declares a body.
    ExternFunctionHasBody {
        /// The function.
        function: FunctionLoc,
    },
    /// `@builtin` is on a function that declares a body.
    ///
    /// What a builtin does is the compiler's, and the declaration is what puts its name in the
    /// scope of the module: a body here is a body nothing calls.
    BuiltinFunctionHasBody {
        /// The function.
        function: FunctionLoc,
    },
    /// `@extern` is on a type, and the language has no external types yet.
    ExternType {
        /// The type.
        class: ClassLoc,
    },
    /// A public function does not declare the type of its result.
    ///
    /// A caller of a public function depends on it, and the surface of the module it is in has
    /// to say what a call of it gives back ([ADR-0004][adr-0004]).
    ///
    /// [adr-0004]: ../../docs/adr/0004-module-system.md
    PublicFunctionWithoutResult {
        /// The function.
        function: FunctionLoc,
    },
    /// A public function has a parameter whose type is not declared.
    PublicParameterWithoutType {
        /// The function.
        function: FunctionLoc,
        /// The name the parameter binds, if it binds one: a parameter that binds none is one
        /// a reader is told about by where it stands, which is the span of the error.
        parameter: Option<Name>,
    },
    /// A name an import brings in is also a name the module declares.
    ///
    /// A name means one thing in a module, and the two say two: what a use of the name means
    /// is the declaration, which the module wrote itself, and the import of it is what a
    /// reader is told about.
    ImportOfDeclaredName {
        /// The name, which is what both of them are about.
        name: Name,
        /// The declaration the name denotes.
        declaration: EntityLoc,
    },
}

impl LoweringError {
    /// The message of the error, in one line.
    ///
    /// Rendering is apart from the value because an error is a fact and a message is what a
    /// caller makes of it: it keeps the fact for as long as it wants and renders it where a
    /// reader reads it. Everything a message is made of is what the error holds; a caller that
    /// knows more than the HIR does --- what the file of a module is called, which language the
    /// reader reads --- renders the [`Diagnostic`] the error becomes, not the error.
    pub fn message(&self) -> String {
        match self {
            Self::DuplicateName { item } => {
                match item.name() {
                    // What the message is about is the name the module wrote, which both of the
                    // declarations wrote: which of the two lost is what the span says.
                    Some(name) => {
                        format!(
                            "`{} {name:?}` is declared more than once in this module",
                            item.kind().keyword(),
                        )
                    },
                    None => format!("`{item:?}` is declared more than once in this module"),
                }
            },
            Self::UnknownAttribute { name } => {
                format!("the language has no attribute `{name:?}`")
            },
            Self::RepeatedAttribute { name } => {
                format!("the attribute `@{name:?}` is written more than once on one declaration")
            },
            Self::DuplicateParameterName { function, name } => {
                format!(
                    "`{function:?}` declares the parameter `{name:?}` more than once, and a \
                 body cannot tell the two apart",
                )
            },
            Self::UnknownEscape { escape } => {
                format!("the language has no escape `{escape}`")
            },
            Self::IntegerLiteralTooLarge { literal } => {
                format!(
                    "the integer literal `{literal}` does not fit the value the HIR holds it in"
                )
            },
            Self::PathIsNotPlain { path } => {
                format!(
                    "`{path}` is not a path of names: the type argument at one of its segments \
                 belongs to a type, and not to a path of the project",
                )
            },
            Self::ExternFunctionHasBody { function } => {
                format!(
                    "`{function:?}` is declared `@extern`, and an external function is implemented \
                 outside the project: it has no body here",
                )
            },
            Self::BuiltinFunctionHasBody { function } => {
                format!(
                    "`{function:?}` is declared `@builtin`, and what a builtin does is the \
                 compiler's: it has no body here",
                )
            },
            Self::ExternType { class } => {
                format!(
                    "`{class:?}` is declared `@extern`, and the language has no external types yet",
                )
            },
            Self::PublicFunctionWithoutResult { function } => {
                format!(
                    "`{function:?}` is public and does not declare the type of its result, \
                 which a caller of it depends on",
                )
            },
            Self::PublicParameterWithoutType {
                function,
                parameter,
            } => {
                let parameter = match parameter {
                    Some(name) => format!("its parameter `{name:?}`"),
                    // A parameter that binds no name --- the wildcard --- is not one a
                    // message can call by a name, and the span is where a reader looks.
                    None => "a parameter that binds no name".to_owned(),
                };

                format!(
                    "the public function `{function:?}` does not declare the type of \
                 {parameter}, which a caller of it depends on",
                )
            },
            Self::ImportOfDeclaredName { name, declaration } => {
                format!(
                    "the name `{name:?}` is imported and declared in this module as `{:?}`: \
                 a use of it means the declaration",
                    declaration.item,
                )
            },
        }
    }
}

impl DiagKind for LoweringError {
    fn level(&self) -> Level {
        Level::Error
    }

    /// Lowering a module is the local half of name resolution ([ADR-0005][adr-0005]): what a
    /// module says about itself is resolved before another module is read, and what it says
    /// wrong is reported here rather than by the pass that follows.
    ///
    /// [adr-0005]: ../../docs/adr/0005-compiler-pipeline.md
    fn category(&self) -> Category {
        Category::Lowering
    }

    /// Two digits of the kind of the error, which stay the same however a message is worded.
    fn code(&self) -> &'static str {
        match self {
            Self::DuplicateName { .. } => "01",
            Self::UnknownAttribute { .. } => "02",
            Self::IntegerLiteralTooLarge { .. } => "03",
            Self::PathIsNotPlain { .. } => "04",
            Self::ExternFunctionHasBody { .. } => "05",
            Self::ExternType { .. } => "06",
            Self::PublicFunctionWithoutResult { .. } => "07",
            Self::PublicParameterWithoutType { .. } => "08",
            Self::BuiltinFunctionHasBody { .. } => "09",
            Self::ImportOfDeclaredName { .. } => "10",
            Self::RepeatedAttribute { .. } => "11",
            Self::DuplicateParameterName { .. } => "12",
            Self::UnknownEscape { .. } => "13",
        }
    }
}

/// A lowering error, and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweringDiag {
    error: LoweringError,
    span: Span,
}

impl LoweringDiag {
    /// An error at `span`.
    pub(crate) fn new(error: LoweringError, span: Span) -> Self {
        Self { error, span }
    }

    /// What lowering found.
    pub fn error(&self) -> &LoweringError {
        &self.error
    }

    /// Where it is.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The diagnostic a host renders: the error, its kind, and where it is.
    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic::from_kind(&self.error, self.error.message()).with_primary(self.span, "")
    }
}

impl fmt::Display for LoweringDiag {
    /// The message of the error, headed by the range it is at.
    ///
    /// The file is [`LoweringDiag::span`] and not part of the text: a caller that renders what
    /// it lowered knows which file it asked about.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.span.range, self.error.message())
    }
}
