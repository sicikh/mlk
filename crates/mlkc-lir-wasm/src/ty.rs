//! The types of the target: what a value of the WASM LIR is ([ADR-0022][adr-0022]).
//!
//! A LIR value is a machine value and not a type of the language ([ADR-0018][adr-0018]): the
//! selection pass turns a word into the representation the WASM backend gives it, and the type
//! of the LIR value is that representation, written down. What the IR names is what the target
//! names --- `i32`, `(ref i31)`, `eqref`, and, as structures and strings are lowered, their GC
//! concrete types.
//!
//! [adr-0018]: ../../docs/adr/0018-values-as-words.md
//! [adr-0022]: ../../docs/adr/0022-wasm-lir.md

use std::fmt;

/// The type of a value of the LIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    /// `i32`: a number the machine computes with.
    I32,
    /// A reference.
    Ref(RefTy),
}

impl Ty {
    /// The type of an immediate: `(ref i31)`.
    pub const I31: Self = Self::Ref(RefTy::I31);

    /// The type of a word: `eqref`, the nullable reference to any `eq` type.
    pub const EQREF: Self = Self::Ref(RefTy::Eq);

    /// Whether the type is a reference.
    pub fn is_ref(self) -> bool {
        matches!(self, Self::Ref(_))
    }

    /// Whether a value of this type is one of `other` as well.
    ///
    /// The subtyping the target has today: an immediate is an `eq` reference, so a value that is
    /// known to be an `i31` passes where a word is read and needs no instruction to cross.
    pub fn is_subtype_of(self, other: Self) -> bool {
        if self == other {
            return true;
        }

        matches!((self, other), (Self::Ref(RefTy::I31), Self::Ref(RefTy::Eq)),)
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I32 => f.write_str("i32"),
            Self::Ref(RefTy::I31) => f.write_str("(ref i31)"),
            Self::Ref(RefTy::Eq) => f.write_str("eqref"),
        }
    }
}

/// The reference types the backend has a name for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefTy {
    /// `(ref i31)`: the number inside a word, and never null.
    I31,
    /// `eqref`: any `eq` reference, `null` included.
    Eq,
}

#[cfg(test)]
mod tests {
    use super::{RefTy, Ty};

    #[test]
    fn an_immediate_is_a_word() {
        assert!(Ty::I31.is_subtype_of(Ty::EQREF));
        assert!(Ty::I31.is_subtype_of(Ty::I31));
        assert!(!Ty::EQREF.is_subtype_of(Ty::I31));
        assert!(!Ty::I31.is_subtype_of(Ty::I32));
        assert!(!Ty::I32.is_subtype_of(Ty::I31));
    }

    #[test]
    fn a_type_reads_as_the_target_writes_it() {
        assert_eq!(Ty::I32.to_string(), "i32");
        assert_eq!(Ty::I31.to_string(), "(ref i31)");
        assert_eq!(Ty::Ref(RefTy::Eq).to_string(), "eqref");
    }
}
