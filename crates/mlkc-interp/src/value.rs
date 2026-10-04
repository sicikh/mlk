//! A word of the language, as the interpreter holds it.
//!
//! A word is uniform ([ADR-0018][adr-0018]): every value the language has is one, and how it is
//! tagged is a property of the back end, not of the meaning. What the interpreter holds is the
//! kind the word has, because the operations of MIR ask for one: arithmetic reads an immediate,
//! a branch reads a condition.
//!
//! [adr-0018]: ../../docs/adr/0018-values-as-words.md

use std::sync::Arc;

use mlkc_hir_def::{EntityLoc, FunctionLoc};
use mlkc_mir::LambdaId;

/// One word.
///
/// The variants are the kinds the language can have of a word today; a structure and a string
/// are words too, and join this enum when the language reads a field or a length ([ADR-0018]).
///
/// [adr-0018]: ../../docs/adr/0018-values-as-words.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A signed 31-bit integer, the payload of an `i31`.
    Int(i32),
    /// A boolean.
    Bool(bool),
    /// The unit value.
    Unit,
    /// A closure: the code of a lambda, and the words it captured ([ADR-0026][adr-0026]).
    ///
    /// [adr-0026]: ../../docs/adr/0026-closure-representation.md
    Closure(Arc<Closure>),
}

/// The code of a lambda and the words it captured, which is what a closure is ([ADR-0026]).
///
/// A lambda is code the body that wrote it holds: the code is addressed by the entity of that
/// body and by the place of the lambda in the body's arena, and the words are what
/// `Rvalue::Capture` reads when the code runs.
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closure {
    /// The body that wrote the lambda.
    pub writer: EntityLoc<FunctionLoc>,
    /// The lambda, in the arena of the writer.
    pub lambda: LambdaId,
    /// The captured words, in the order the closure was created.
    pub captures: Vec<Value>,
}

impl Value {
    /// The payload of the immediate the word is, if it is one.
    ///
    /// This is the value `i31.get_s` reads in the WASM back end, and it is what an operator
    /// over immediates computes with: a boolean is `0` or `1`, and the unit is `0`.
    pub fn immediate(&self) -> Option<i32> {
        match self {
            Self::Int(value) => Some(*value),
            Self::Bool(value) => Some(i32::from(*value)),
            Self::Unit => Some(0),
            // A closure is a reference: it is not an immediate, whatever it captured.
            Self::Closure(_) => None,
        }
    }

    /// Whether the word is the true boolean, the way a branch reads one.
    ///
    /// The condition of a branch is a word of the language: the WASM back end reads it with
    /// `i31.get_s` and tests it against zero, and so does the interpreter.
    pub fn truth(&self) -> bool {
        self.immediate().is_some_and(|payload| payload != 0)
    }
}
