//! What the construction of MIR reports: the errors of a body, and where they are.
//!
//! An error is a value, not a sentence ([ADR-0005][adr-0005]): the lowering of a body reports
//! only what the checker could not --- a literal outside the range of the representation, a
//! construct the language does not have yet --- and the facts it holds are what a message is
//! rendered from.
//!
//! [adr-0005]: ../../docs/adr/0005-compiler-pipeline.md

use std::fmt;

use mlkc_diagnostics::{Category, DiagKind, Diagnostic, Level};
use mlkc_hir_def::{BinaryOp, Name, UnaryOp};
use mlkc_hir_ty::Ty;
use mlkc_span::Span;

/// What an operator of the HIR is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    /// A binary operator.
    Binary(BinaryOp),
    /// A unary operator.
    Unary(UnaryOp),
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binary(op) => op.fmt(f),
            Self::Unary(op) => op.fmt(f),
        }
    }
}

/// What the lowering of a body found that the check did not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirError {
    /// An `Int` literal does not fit the 31-bit representation of [ADR-0018][adr-0018].
    ///
    /// [adr-0018]: ../../docs/adr/0018-values-as-words.md
    IntOutOfRange {
        /// The literal as the module wrote it.
        value: i64,
    },
    /// The HIR holds no node where one belongs: broken syntax the check absorbed.
    Missing,
    /// A field read: the language has no fields yet.
    Field {
        /// The name written after the operator.
        name: Name,
    },
    /// A path denotes an entity, and the language has no value for it yet.
    NotAValue {
        /// The name the path is rooted at.
        name: Name,
    },
    /// A path the check read resolved to nothing.
    Unresolved {
        /// The name the path is rooted at.
        name: Name,
    },
    /// A callee is not a function.
    NotCallable {
        /// The name the callee is.
        name: Name,
    },
    /// Equality of a type the language has no comparison for yet.
    Equality {
        /// The type of the operands.
        ty: Ty,
    },
    /// An operator of operands the language has no primitive for.
    Operator {
        /// The operator.
        op: Operator,
    },
    /// The check left no type for an operand of an operator.
    MissingType,
}

impl MirError {
    /// The message of the error, in one line.
    pub fn message(&self) -> String {
        match self {
            Self::IntOutOfRange { value } => {
                format!("the integer literal `{value}` is outside the 31-bit range of `Int`",)
            },
            Self::Missing => {
                "an expression is missing here, and the parser is what says so".to_owned()
            },
            Self::Field { name } => {
                format!("the language has no fields, and `{name:?}` is read as one")
            },
            Self::NotAValue { name } => {
                format!("`{name:?}` denotes an entity, which the language has no value for yet")
            },
            Self::Unresolved { name } => format!("the name `{name:?}` denotes nothing here"),
            Self::NotCallable { name } => {
                format!("`{name:?}` is called, and it is not a function")
            },
            Self::Equality { ty } => {
                format!("the language has no comparison for two values of `{ty}` yet")
            },
            Self::Operator { op } => {
                format!("the language has no primitive for `{op}` on these operands")
            },
            Self::MissingType => "the check left no type for an operand here".to_owned(),
        }
    }
}

impl DiagKind for MirError {
    fn level(&self) -> Level {
        Level::Error
    }

    /// MIR is the stage between the check and the back end ([ADR-0019][adr-0019]).
    ///
    /// [adr-0019]: ../../docs/adr/0019-mir.md
    fn category(&self) -> Category {
        Category::Mir
    }

    /// Two digits of the kind of the error, which stay the same however a message is worded.
    fn code(&self) -> &'static str {
        match self {
            Self::IntOutOfRange { .. } => "01",
            Self::Missing => "02",
            Self::Field { .. } => "03",
            Self::NotAValue { .. } => "04",
            Self::Unresolved { .. } => "05",
            Self::NotCallable { .. } => "06",
            Self::Equality { .. } => "07",
            Self::Operator { .. } => "08",
            Self::MissingType => "09",
        }
    }
}

/// An error of the construction of MIR, and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirDiag {
    error: MirError,
    span: Span,
}

impl MirDiag {
    /// An error at `span`.
    pub(crate) fn new(error: MirError, span: Span) -> Self {
        Self { error, span }
    }

    /// What the construction found.
    pub fn error(&self) -> &MirError {
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

impl fmt::Display for MirDiag {
    /// The message of the error, headed by the range it is at.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.span.range, self.error.message())
    }
}
