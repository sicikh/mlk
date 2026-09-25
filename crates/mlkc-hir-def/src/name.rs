//! A name, as declarations and references spell it.

use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
};

use mlkc_intern::{Symbol, sym};

/// A name in the HIR, used for both a declaration and a reference.
///
/// # Equality, hashing, and ordering
///
/// The name is interned, so `Eq` is pointer equality and is exact
/// as long as the interner keeps one live allocation per text
/// and a compared value stays alive -- which the driver guarantees
/// by holding the value its key names.
///
/// `Hash` and `Ord` are by text, never by address:
/// a map layout, a sorted list, or a list of use sites in a diagnostic
/// must not depend on where an allocation happened to land.
///
/// A textual fallback in `Eq` is deliberately absent:
/// it would make the cost of equality depend on the length of a name
/// and would hide a violation of the retention rule instead of surfacing it.
#[derive(Clone)]
pub struct Name {
    symbol: Symbol,
}

impl Name {
    /// Interns a name.
    pub fn new(text: &str) -> Self {
        Self {
            symbol: Symbol::intern(text),
        }
    }

    /// The name of a declaration or a reference that is missing.
    pub const fn missing() -> Self {
        Self {
            symbol: sym::missing_name,
        }
    }

    /// Whether this is the name of something that is not there.
    pub fn is_missing(&self) -> bool {
        self.symbol == sym::missing_name
    }

    /// The text of the name.
    pub fn as_str(&self) -> &str {
        self.symbol.as_str()
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol
    }
}

impl Eq for Name {}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for Name {
    /// The text of the name, or a word for a name that is not there.
    ///
    /// A name that is not there is a name like any other inside the interner, and what it is
    /// written as is a sentinel; a reader of the HIR is told what it means instead.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_missing() {
            f.write_str("<missing>")
        } else {
            f.write_str(self.as_str())
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    fn hash(name: &Name) -> u64 {
        let mut state = DefaultHasher::new();
        name.hash(&mut state);
        state.finish()
    }

    #[test]
    fn the_same_text_is_the_same_name() {
        let first = Name::new("foo");
        let second = Name::new("foo");

        assert_eq!(first, second);
        assert_eq!(hash(&first), hash(&second));
    }

    #[test]
    fn a_name_hashes_and_orders_by_text() {
        let mut names = [Name::new("b"), Name::new("c"), Name::new("a")];
        names.sort();

        assert_eq!(names.iter().map(Name::as_str).collect::<Vec<_>>(), [
            "a", "b", "c"
        ],);
        // The hash is a function of the text alone, so it does not change with the allocation.
        assert_eq!(hash(&Name::new("foo")), hash(&Name::new("foo")));
    }

    #[test]
    fn a_missing_name_is_the_missing_name() {
        assert!(Name::missing().is_missing());
        assert!(!Name::new("foo").is_missing());

        // The text of it is a sentinel of the interner, and what it is read as is a word.
        assert_eq!(format!("{:?}", Name::missing()), "<missing>");
        assert_eq!(format!("{:?}", Name::new("foo")), "foo");
    }
}
