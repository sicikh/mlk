use mlkc_intern::{Symbol, sym};

/// `Name` is a wrapper around string used in HIR for both references and declarations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name {
    symbol: Symbol,
}

impl Name {
    pub fn as_str(&self) -> &str {
        self.symbol.as_str()
    }

    pub const fn symbol(&self) -> &Symbol {
        &self.symbol
    }

    pub fn new(text: &str) -> Name {
        Name {
            symbol: Symbol::intern(text),
        }
    }

    pub const fn missing() -> Name {
        Name {
            symbol: sym::missing_name,
        }
    }

    pub fn is_missing(&self) -> bool {
        self == &Name::missing()
    }

    pub fn new_generated(idx: usize) -> Self {
        Name::new(&format!("<wc@gennew>{idx}"))
    }
}

impl PartialEq<Symbol> for Name {
    fn eq(&self, symbol: &Symbol) -> bool {
        self.symbol == *symbol
    }
}

impl PartialEq<&Symbol> for Name {
    fn eq(&self, symbol: &&Symbol) -> bool {
        self.symbol == **symbol
    }
}

impl PartialEq<Name> for Symbol {
    fn eq(&self, name: &Name) -> bool {
        *self == name.symbol
    }
}

impl PartialEq<Name> for &Symbol {
    fn eq(&self, name: &Name) -> bool {
        **self == name.symbol
    }
}

pub trait AsName {
    fn as_name(&self) -> &Name;
}
