//! What a check found, and where it is.
//!
//! An error is a value, not a sentence: the facts are the types and the names the HIR knows, and
//! where the error is is a place in the HIR --- an expression, a pattern, or a type a declaration
//! writes. The driver turns that into a span with the ranges it holds for a host ([ADR-0009]).
//!
//! [ADR-0009]: ../../docs/adr/0009-pass-contract.md

use std::fmt;

use mlkc_diagnostics::{Category, DiagKind, Diagnostic, Level};
use mlkc_hir_def::{ExprId, FunctionLoc, ItemLoc, Name, PatId, dump::TypePlace as DeclaredType};
use mlkc_hir_ty::Ty;
use mlkc_resolve::ResolveError;
use mlkc_span::Span;

/// What a check could not do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    /// A top-level declaration does not write a type the first check needs.
    ///
    /// Every signature of a top-level declaration is written while a body is checked on its own:
    /// inferring a signature from a body would need an order between the bodies of a module
    /// ([ADR-0017]).
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    MissingType {
        /// The function whose signature is incomplete.
        function: FunctionLoc,
        /// Which parameter is not written, or `None` for the result.
        parameter: Option<usize>,
    },
    /// A name written where a type belongs denotes something else.
    NotAType {
        /// The name written where a type belongs.
        name: Name,
    },
    /// A name written where a value belongs denotes something else.
    NotAValue {
        /// The name written where a value belongs.
        name: Name,
    },
    /// A name is applied to type arguments, and the language has no generics yet.
    TypeArguments {
        /// The name the arguments are applied to.
        name: Name,
    },
    /// A name is written inside a type or a value, and the language has no members yet.
    NestedName {
        /// The name that is written inside something.
        name: Name,
    },
    /// A value is called, and it is not a function.
    NotCallable {
        /// The type of the value the call calls.
        found: Ty,
    },
    /// A call passes a number of arguments the function does not take.
    ArgumentCount {
        /// How many arguments the function takes.
        expected: usize,
        /// How many the call passes.
        found: usize,
    },
    /// A body binds a number of parameters other than the declaration takes.
    ParameterCount {
        /// How many parameters the declaration takes.
        expected: usize,
        /// How many the body binds.
        found: usize,
    },
    /// Two types that have to be one are two.
    TypeMismatch {
        /// The type a place expects.
        expected: Ty,
        /// The type of what is written there.
        found: Ty,
    },
    /// A type would contain itself.
    RecursiveType,
    /// A path of a body does not resolve: the module it names holds no such name, no module of
    /// the project is named by it, or a chain of re-exports returns to itself.
    ///
    /// The paths of the surface of a module are walked by its resolution, which reports what it
    /// finds; this is the check telling about a path a resolution does not read ([ADR-0016]).
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    Unresolved {
        /// What the walk found wrong.
        error: ResolveError,
    },
    /// The check of a body found no type for the declaration that owns it.
    ///
    /// This is a bug of the input the driver assembled, and not of the module: the types of a
    /// module are resolved before its bodies are checked ([ADR-0017]).
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    MissingSignature,
    /// An integer literal does not fit the 31-bit representation of `Int` ([ADR-0018]).
    ///
    /// The range of a literal is the meaning of the type it is written with, so it is the check
    /// that reports it, and the construction of MIR reads it as an invariant.
    ///
    /// [ADR-0018]: ../../docs/adr/0018-values-as-words.md
    IntOutOfRange {
        /// The literal as the module wrote it.
        value: i64,
    },
    /// Equality is applied to values whose type the language has no equality for yet.
    NoEquality {
        /// The type of the operands.
        ty: Ty,
    },
    /// A lambda is written, and the language has no value of a function yet.
    ///
    /// The lambda is lowered into the HIR with the bindings its body captures; what turns one
    /// into a value is the representation of a closure, which the language has yet to fix
    /// ([ADR-0018][adr-0018]).
    ///
    /// [adr-0018]: ../../docs/adr/0018-values-as-words.md
    Lambda,
}

impl TypeError {
    /// The message of the error, in one line.
    pub fn message(&self) -> String {
        match self {
            Self::MissingType {
                function,
                parameter: Some(index),
            } => {
                format!("`{function:?}` does not declare the type of its parameter #{index}")
            },
            Self::MissingType {
                function,
                parameter: None,
            } => format!("`{function:?}` does not declare the type of its result"),
            Self::NotAType { name } => {
                format!("the name `{name:?}` does not denote a type, and a type belongs here")
            },
            Self::NotAValue { name } => {
                format!("the name `{name:?}` does not denote a value, and a value belongs here")
            },
            Self::TypeArguments { name } => {
                format!("the name `{name:?}` is applied to type arguments")
            },
            Self::NestedName { name } => {
                format!("`{name:?}` is a name written inside something")
            },
            Self::NotCallable { found } => {
                format!("a value of type `{found}` is called, and a function is what a call calls")
            },
            Self::ArgumentCount { expected, found } => {
                format!(
                    "the function takes {}, and the call passes {}",
                    count(*expected, "argument"),
                    count(*found, "argument"),
                )
            },
            Self::ParameterCount { expected, found } => {
                format!(
                    "the declaration takes {}, and the body binds {}",
                    count(*expected, "parameter"),
                    count(*found, "parameter"),
                )
            },
            Self::TypeMismatch { expected, found } => {
                format!("a value of type `{found}` is where a value of type `{expected}` belongs")
            },
            Self::RecursiveType => "the type of the value would contain itself".to_owned(),
            Self::Unresolved { error } => error.message(),
            Self::MissingSignature => {
                "the body was checked without the type of the declaration that owns it, and the \
                 types of a module are resolved before its bodies are checked"
                    .to_owned()
            },
            Self::IntOutOfRange { value } => {
                format!("the integer literal `{value}` is outside the 31-bit range of `Int`")
            },
            Self::NoEquality { ty } => {
                format!("the language has no equality for two values of `{ty}` yet")
            },
            Self::Lambda => {
                "a lambda is a value of a function, and the language has no value of a function \
                 yet"
                .to_owned()
            },
        }
    }

    /// What a host writes under the place it marks, in a few words.
    ///
    /// The message is the headline of the error;
    /// the label is the part of it that belongs to the place,
    /// which is what an editor shows where the caret stands.
    pub fn label(&self) -> String {
        match self {
            Self::MissingType {
                parameter: Some(_), ..
            } => "the type of this parameter is not written".to_owned(),
            Self::MissingType {
                parameter: None, ..
            } => "the type of this result is not written".to_owned(),
            Self::NotAType { .. } => "a type belongs here".to_owned(),
            Self::NotAValue { .. } => "a value belongs here".to_owned(),
            Self::TypeArguments { .. } => "a class takes no arguments".to_owned(),
            Self::NestedName { .. } => "the language has no members yet".to_owned(),
            Self::NotCallable { found } => format!("`{found}` is not a function"),
            Self::ArgumentCount { expected, found } => {
                format!(
                    "expected {}, found {}",
                    count(*expected, "argument"),
                    count(*found, "argument"),
                )
            },
            Self::ParameterCount { expected, found } => {
                format!(
                    "expected {}, found {}",
                    count(*expected, "parameter"),
                    count(*found, "parameter"),
                )
            },
            Self::TypeMismatch { expected, found } => {
                format!("expected `{expected}`, found `{found}`")
            },
            Self::RecursiveType => "this type contains itself".to_owned(),
            Self::IntOutOfRange { .. } => "outside the range of `Int`".to_owned(),
            Self::NoEquality { .. } => "this type has no equality yet".to_owned(),
            Self::Lambda => "the language has no value of a function yet".to_owned(),
            Self::Unresolved { .. } | Self::MissingSignature => String::new(),
        }
    }

    /// What the error has to add beyond the place it is at: a reason, a hint, a next step.
    pub fn notes(&self) -> Vec<String> {
        match self {
            Self::MissingType { .. } => {
                vec![
                    "the first check does not infer the signature of a top-level function: every \
                 top-level declaration writes its types, and inference of them is deferred"
                        .to_owned(),
                ]
            },
            Self::TypeArguments { .. } => {
                vec!["the language has no generics yet".to_owned()]
            },
            Self::IntOutOfRange { .. } => {
                vec!["`Int` is limited to 31 bits until boxed integers are designed".to_owned()]
            },
            Self::NoEquality { .. } => {
                vec!["`Int` and `Bool` are the types with equality for now".to_owned()]
            },
            Self::Lambda => {
                vec![
                    "the translation of a lambda is deferred: the HIR holds the bindings its \
                     body captures, and the stages after the check do not read one yet"
                        .to_owned(),
                ]
            },
            _ => Vec::new(),
        }
    }
}

/// A count and the word it counts, in the number the count is: `1 argument`, `2 arguments`.
fn count(count: usize, thing: &str) -> String {
    if count == 1 {
        format!("{count} {thing}")
    } else {
        format!("{count} {thing}s")
    }
}

impl DiagKind for TypeError {
    fn level(&self) -> Level {
        Level::Error
    }

    /// Checking types is the stage after name resolution ([ADR-0005]).
    ///
    /// [ADR-0005]: ../../docs/adr/0005-compiler-pipeline.md
    fn category(&self) -> Category {
        Category::TypeChecker
    }

    /// Two digits of the kind of the error, which stay the same however a message is worded.
    fn code(&self) -> &'static str {
        match self {
            Self::MissingType { .. } => "01",
            Self::NotAType { .. } => "02",
            Self::NotAValue { .. } => "03",
            Self::TypeArguments { .. } => "04",
            Self::NestedName { .. } => "05",
            Self::NotCallable { .. } => "06",
            Self::ArgumentCount { .. } => "07",
            Self::ParameterCount { .. } => "08",
            Self::TypeMismatch { .. } => "09",
            Self::RecursiveType => "10",
            Self::Unresolved { .. } => "11",
            Self::MissingSignature => "12",
            Self::IntOutOfRange { .. } => "13",
            Self::NoEquality { .. } => "14",
            Self::Lambda => "15",
        }
    }
}

/// Where a check error is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypePlace {
    /// An expression of a body.
    Expr(ExprId),
    /// A pattern of a body.
    Pat(PatId),
    /// An entity of a module as a whole, such as the body of a declaration.
    Entity(ItemLoc),
    /// A type a declaration writes: the entity, and which type of it.
    Declared {
        /// The declaration the type is written in.
        item: ItemLoc,
        /// Which type of the declaration it is.
        place: DeclaredType,
    },
}

impl TypePlace {
    /// The expression the place is, if it is one.
    pub fn expr(&self) -> Option<ExprId> {
        match self {
            Self::Expr(id) => Some(*id),
            _ => None,
        }
    }

    /// The pattern the place is, if it is one.
    pub fn pat(&self) -> Option<PatId> {
        match self {
            Self::Pat(id) => Some(*id),
            _ => None,
        }
    }

    /// The entity the place is in, if it is in one.
    pub fn item(&self) -> Option<&ItemLoc> {
        match self {
            Self::Expr(_) | Self::Pat(_) => None,
            Self::Entity(item) | Self::Declared { item, .. } => Some(item),
        }
    }

    /// Which type of the entity the place is, if it is a type a declaration writes.
    pub fn declared(&self) -> Option<DeclaredType> {
        match self {
            Self::Declared { place, .. } => Some(*place),
            _ => None,
        }
    }
}

/// A check error, and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDiag {
    error: TypeError,
    place: TypePlace,
}

impl TypeDiag {
    /// An error at `place`.
    pub(crate) fn new(error: TypeError, place: TypePlace) -> Self {
        Self { error, place }
    }

    /// What the check found.
    pub fn error(&self) -> &TypeError {
        &self.error
    }

    /// Where it is.
    pub fn place(&self) -> &TypePlace {
        &self.place
    }

    /// The diagnostic a host renders: the error, its kind, the span of the place, and the words
    /// that belong under it.
    ///
    /// The span is the driver's: a pass knows the HIR and not the file, and where a node of the
    /// HIR is written is what the driver holds ([ADR-0009]).
    ///
    /// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
    pub fn to_diagnostic(&self, span: Span) -> Diagnostic {
        self.to_diagnostic_with(span, self.error.message())
    }

    /// The diagnostic a host renders, with the message the caller reads the error by.
    ///
    /// A check of a body walks the paths of the body itself, and a name a walk could not find
    /// may be a name a module keeps to itself: the driver tells that by rendering the error with
    /// the message of the look rather than the message of the walk ([ADR-0017]).
    ///
    /// [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    pub fn to_diagnostic_with(&self, span: Span, message: impl Into<String>) -> Diagnostic {
        let mut diagnostic =
            Diagnostic::from_kind(&self.error, message).with_primary(span, self.error.label());

        for note in self.error.notes() {
            diagnostic = diagnostic.with_note(note);
        }

        diagnostic
    }
}

impl fmt::Display for TypeDiag {
    /// The message of the error, headed by where it is.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.place {
            TypePlace::Expr(id) => {
                write!(f, "expr #{}: {}", id.into_raw(), self.error.message())
            },
            TypePlace::Pat(id) => {
                write!(f, "pat #{}: {}", id.into_raw(), self.error.message())
            },
            TypePlace::Entity(item) => write!(f, "{item:?}: {}", self.error.message()),
            TypePlace::Declared { item, place } => {
                write!(f, "{item:?} [{place:?}]: {}", self.error.message())
            },
        }
    }
}
