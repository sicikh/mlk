use std::str::FromStr;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

use crate::{kind_src::KindsSrc, mlk_kinds_src::MLK_KINDS_SRC};

pub const LANGUAGE_PREFIXES: [&str; 1] = ["mlk_"];

#[derive(Debug, Eq, Copy, Clone, PartialEq)]
pub enum LanguageKind {
    Mlk,
}

impl std::fmt::Display for LanguageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Mlk => write!(f, "mlk"),
        }
    }
}

pub const ALL_LANGUAGE_KIND: [LanguageKind; 1] = [LanguageKind::Mlk];

impl FromStr for LanguageKind {
    type Err = String;

    fn from_str(kind: &str) -> Result<Self, Self::Err> {
        match kind {
            "mlk" => Ok(Self::Mlk),
            _ => Err(format!("Language {kind} not supported, please use: `mlk`")),
        }
    }
}

/// A helper macro to make it easier to define functions that return tokens for a specific language kind.
macro_rules! define_language_kind_function {
    ([$($kind:ident),*],$func:ident,$out:expr) => {
        pub(crate) fn $func(&self) -> TokenStream {
            match self {
                $( LanguageKind::$kind => {
                    // HACK: workaround for $kind$out adding an extra space between the two
                    // let ident = format_ident!("{}{}", stringify!($kind), stringify!($out));
                    let ident = format_ident!("{}", stringify!($out));
                    quote! { #ident }
                },)*
            }
        }
    }
}

/// A helper macro to define functions for each language kind to make it slightly less tedious to add new languages.
macro_rules! define_language_kind_functions {
    ([$($kind:ident),*]) => {
        define_language_kind_function!([$($kind),*], syntax_kind, SyntaxKind);
        define_language_kind_function!([$($kind),*], syntax_factory, SyntaxFactory);
        define_language_kind_function!([$($kind),*], syntax_node, SyntaxNode);
        define_language_kind_function!([$($kind),*], syntax_element, SyntaxElement);
        define_language_kind_function!([$($kind),*], syntax_token, SyntaxToken);
        define_language_kind_function!([$($kind),*], syntax_element_children, SyntaxElementChildren);
        define_language_kind_function!([$($kind),*], syntax_list, SyntaxList);
    }
}

impl LanguageKind {
    define_language_kind_functions!([Mlk]);

    pub(crate) fn syntax_crate_ident(&self) -> Ident {
        Ident::new("mlkc_syntax", Span::call_site())
    }

    pub fn kinds(&self) -> KindsSrc<'_> {
        match self {
            Self::Mlk => MLK_KINDS_SRC,
        }
    }

    pub fn load_grammar(&self) -> &'static str {
        match self {
            Self::Mlk => include_str!("../mlk.ungram"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ALL_LANGUAGE_KIND, LanguageKind};

    #[test]
    fn language_kind_parses_its_name() {
        assert_eq!("mlk".parse::<LanguageKind>(), Ok(LanguageKind::Mlk));
    }

    #[test]
    fn an_unknown_language_name_is_rejected_and_the_supported_one_named() {
        let error = "css".parse::<LanguageKind>().unwrap_err();

        assert!(error.contains("not supported"), "{error}");
        assert!(error.contains("`mlk`"), "{error}");
    }

    #[test]
    fn every_language_kind_round_trips_through_its_display_name() {
        for kind in ALL_LANGUAGE_KIND {
            assert_eq!(kind.to_string().parse::<LanguageKind>(), Ok(kind));
        }
    }

    #[test]
    fn every_language_kind_has_a_grammar_and_kinds() {
        for kind in ALL_LANGUAGE_KIND {
            assert!(!kind.load_grammar().is_empty());
            assert!(!kind.kinds().nodes.is_empty());
        }
    }
}
