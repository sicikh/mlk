# An import is a tree of names, and `self` is the path itself

- Status: accepted
- Date: 2026-10-16

## Context and Problem Statement

An import names one thing:
`use std::core::Int`, `use std::core::Int as Integer`, `use std::core::io as IO`.
A module that takes five names from one place writes the path to it five times,
and the lines differ in one word.
The prefix is not information:
a reader diffs two imports to see what a module takes,
and the vocabulary a tree is written with is already there ---
`use`, `as`, and the `::` separator ([ADR-0012][0012-split-the-dot-operator.md]).

The question: may one import name several things at once,
and how is the path itself written, rather than a name inside it?

## Decision Drivers

- A path is written once, however many names are taken from it.
- What a module takes is read where it is written:
  the tree is what a person writes by hand, and not a form a tool expands.
- The meaning of an import stays independent of the contents of another module
  ([ADR-0004][0004-module-system.md]):
  a tree is sugar for the imports it holds, and every one of them is still a name.
- The surface is the one users know from Rust,
  where an import tree is written with `{...}` and `self`.
- Sugar stops at lowering ([ADR-0014][0014-syntactic-sugar.md]):
  the HIR holds one import per name, and a tree is a way to write several of them together.
- A host points at what it reports:
  a name that resolves to nothing is reported where it is written,
  which is what a tree can say and a whole declaration cannot.

## Considered Options

- **One name per import** --- the surface as it is.
- **A tree of groups, and the keyword `self` for the path itself** --- the chosen form.
- **A tree of groups, and a trailing `::self`** --- the same tree, another spelling of the path.
- **A glob, `use std::core::*`** --- refused by [ADR-0004][0004-module-system.md],
  and not reopened here.

## Decision Outcome

Chosen option: "A tree of groups, and the keyword `self` for the path itself".

### The rule

An import is written as a tree of names:

```
use std::core::Int

use std::core::Int as Integer

use std::{collections::{Map, Set}, prelude::{self, Int, String}}
```

- a node of a tree is a path, and what is taken from it;
- a group `::{...}` holds the names taken from the path before it, separated by `,`;
- a name of a group is a path of its own, read inside the path before it:
  `use std::{core::Int}` takes `std::core::Int`;
- the keyword `self` is the path the group is written after:
  `use std::prelude::{self, Int}` takes the module `std::prelude` itself,
  and the name `Int` from inside it;
- a name is brought in under another one by an `as` of its own,
  written inside the group where the name is: `use std::{Int as Integer}`;
- a group that holds nothing, `use a::{}`, takes nothing, and is read as written.

Every leaf of the tree is an import of its own ([ADR-0014][0014-syntactic-sugar.md]).
The name it brings in is the last name of its path, or the name it is renamed to;
the visibility in front of the `use` is the visibility of every leaf;
and the leaves are imports of the module in the order they are written,
which is the order a reader reads them in.

The surface is read by the parser of [mlkc-parser] and shaped by the grammar of [mlkgr],
and it is sugar to the lowering of [mlkc-lower],
which declares one entity per leaf and keeps no tree in the HIR.
Two spellings are refused where they are written:
`self` outside a group, as in `use std::prelude::self`,
where a name is what the path of a leaf is made of;
and an `as` after a group, `use a::{b} as c`,
where what is renamed is one name and a group holds several.

### What a host reads

A name of a tree is written at a node of its own,
and the position the item tree records for the entity of that name is that node
(the `ItemSyntaxLoc` of [mlkc-hir-def]).
A diagnostic about an import therefore points at the name a reader wrote,
and not at the import around it,
which is what the driver's report of a name nothing holds shows:

```
use std::{collections::{Map, Set}, prelude::{self, Int, String}}
                        ^    ^
```

A whole declaration is what a `UseDecl` node was,
and a reader of a mistake in a tree of five names would be pointed at all five.

### Positive Consequences

- A path is written once, and a tree reads as the set of names a module takes.
- Every name is an import of its own in the HIR:
  resolution, re-export, visibility, and the modules an interface depends on are what they were
  ([ADR-0004][0004-module-system.md], [ADR-0016][0016-inter-module-resolution.md]).
- A diagnostic points at the name, which is where a reader looks.

### Negative Consequences

- The surface has more grammar:
  a group, a name that takes names from inside itself, and the keyword `self`.
- `self` is now a keyword, and no name of a module may be written with it.
- The position of an import in the syntax descends into the tree,
  so the path of child slots an entity is recorded at is longer than it was,
  and a host that holds one holds a longer path.
- A reader of a language without import trees meets a new construct,
  and a reader of Rust has to learn that `self` is written among the names of a group
  and not after a `::`.

## Pros and Cons of the Options

### One name per import

- Good, because nothing is added to the language, and every import reads as one line.
- Good, because an import is one entity and one place, with no expansion to explain.
- Bad, because the prefix of a set of imports is written once per name,
  and a diff of two imports of one module shows one word.
- Bad, because a module cannot grow to take a name from a place it already takes names from
  without repeating the path to it.

### A tree of groups, and a `self` among the names

Chosen. Described above.

- Good, because a path is written once and the names taken from it are read as a set.
- Good, because the HIR and every stage after the lowering are untouched:
  the tree is sugar that stops at the end of the lowering ([ADR-0014][0014-syntactic-sugar.md]).
- Good, because the spelling is the one users know from Rust,
  which keeps the language readable to a reader who has seen a use tree before.
- Bad, because the lexer gains a keyword, which no name may carry.

### A tree of groups, and a trailing `::self`

- Good, because the path itself is then written where the rest of the path is:
  `use std::prelude::self` names `std::prelude` and takes it.
- Bad, because `self` then stands where a name of a path stands,
  and a path is a sequence of names everywhere else in the language:
  the parser reads one keyword apart in a place where one rule is written.
- Bad, because Rust refuses the spelling,
  and a reader who knows use trees expects the two forms to mean the two things they mean there.

### A glob

- Good, because taking everything a module holds is one line.
- Bad, because a name another module holds would become a name of this one
  without either module writing it, which the module system refuses
  ([ADR-0004][0004-module-system.md]):
  what a module takes is what it wrote.

## Links

- The module system, and why an import is named: [0004-module-system.md]
- The separator a path and a tree are written with: [0012-split-the-dot-operator.md]
- Sugar that stops at the lowering: [0014-syntactic-sugar.md]
- Resolution against interfaces, which a tree does not change: [0016-inter-module-resolution.md]
- Implementation: the grammar of [mlkgr], the parser of [mlkc-parser], the lowering of [mlkc-lower],
  and the editor's [grammar]

[0004-module-system.md]: 0004-module-system.md
[0012-split-the-dot-operator.md]: 0012-split-the-dot-operator.md
[0014-syntactic-sugar.md]: 0014-syntactic-sugar.md
[0016-inter-module-resolution.md]: 0016-inter-module-resolution.md
[mlkgr]: ../../xtask/codegen/mlk.ungram
[mlkc-parser]: ../../crates/mlkc-parser
[mlkc-lower]: ../../crates/mlkc-lower
[mlkc-hir-def]: ../../crates/mlkc-hir-def
[grammar]: ../../web/src/lib/grammar/mlk.grammar
