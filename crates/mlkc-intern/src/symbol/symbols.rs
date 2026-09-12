//! Module defining all known symbols required by the rest of rust-analyzer.
#![allow(non_upper_case_globals)]

use std::hash::{BuildHasher, BuildHasherDefault};

use dashmap::{DashMap, SharedValue};
use rustc_hash::FxHasher;

use crate::{Symbol, symbol::TaggedArcPtr};

macro_rules! define_symbols {
    (@WITH_NAME: $($alias:ident = $value:literal,)* @PLAIN: $($name:ident,)*) => {
        // The strings should be in `static`s so that symbol equality holds.
        $(
            pub const $name: Symbol = {
                static SYMBOL_STR: &str = stringify!($name);
                Symbol { repr: TaggedArcPtr::non_arc(&SYMBOL_STR) }
            };
        )*
        $(
            pub const $alias: Symbol = {
                static SYMBOL_STR: &str = $value;
                Symbol { repr: TaggedArcPtr::non_arc(&SYMBOL_STR) }
            };
        )*


        pub(super) fn prefill() -> DashMap<Symbol, (), BuildHasherDefault<FxHasher>> {
            let mut dashmap_ = <DashMap<Symbol, (), BuildHasherDefault<FxHasher>>>::with_hasher(BuildHasherDefault::default());

            let hasher_ = dashmap_.hasher().clone();
            let hash_one = |it_: &str| hasher_.hash_one(it_);
            {
                $(
                    let s = stringify!($name);
                    let hash_ = hash_one(s);
                    let shard_idx_ = dashmap_.determine_shard(hash_ as usize);
                    dashmap_.shards_mut()[shard_idx_].get_mut().insert(hash_, ($name, SharedValue::new(())), |(x, _)| hash_one(x.as_str()));
                )*
                $(
                    let s = $value;
                    let hash_ = hash_one(s);
                    let shard_idx_ = dashmap_.determine_shard(hash_ as usize);
                    dashmap_.shards_mut()[shard_idx_].get_mut().insert(hash_, ($alias, SharedValue::new(())), |(x, _)| hash_one(x.as_str()));
                )*
            }
            dashmap_
        }
    };
}
define_symbols! {
    @WITH_NAME:

    INTEGER_0 = "0",
    INTEGER_1 = "1",
    INTEGER_2 = "2",
    INTEGER_3 = "3",
    INTEGER_4 = "4",
    INTEGER_5 = "5",
    INTEGER_6 = "6",
    INTEGER_7 = "7",
    INTEGER_8 = "8",
    INTEGER_9 = "9",
    INTEGER_10 = "10",
    INTEGER_11 = "11",
    INTEGER_12 = "12",
    INTEGER_13 = "13",
    INTEGER_14 = "14",
    INTEGER_15 = "15",
    __empty = "",
    let_ = "let",
    missing_name = "<wc@missing-name>",

    @PLAIN:
    fun,
}
