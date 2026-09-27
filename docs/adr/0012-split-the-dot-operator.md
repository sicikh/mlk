# Split the `.` operator into field access, UFCS, and namespace access

- Status: accepted
- Date: 2026-09-27

## Context and Problem Statement

MLK has one `.`, and it does one thing:
it qualifies a segment of a path, as in `project.data.start-app` and `std.core.Int`.
Two more uses of the same character are expected of the language:
reading a field, which would be `data.x`,
and calling a function with a value as its first argument
(UFCS — uniform function call syntax), which would be `value.function(args)`.
The question is whether one character may keep meaning all three,
or whether each meaning gets a spelling of its own.

The three uses are not three spellings of one operation.
Each is resolved by a different stage,
and each of those stages reads a different input:

- a path is resolved against the import and export tables of the project
  by the global pass ([ADR-0004][0004-module-system.md], [ADR-0005][0005-compiler-pipeline.md]);
- a field is read from the type of the receiver,
  which type checking has and name resolution does not;
- a call with the receiver as its first argument anchors at the name or the path written before the dot,
  which name resolution reads — the module's own scope for a name, the tables of the project for a path —
  before any type is known ([ADR-0010][0010-stable-entity-identity.md], [ADR-0011][0011-module-prelude.md]).

Overloading one character with all three
makes the meaning of `value.name` a question about types and global tables
rather than about the text, with a cost at every stage:

- the parser must commit to a node kind at parse time ([ADR-0002][0002-lossless-syntax-tree.md]),
  and no kind fits: a path segment, a field access, and a call with an implicit argument
  have different children.
  Either the tree keeps one shape and is rewritten once the world is known
  — and then the tree is no longer a function of the text,
  which is what the identity of a parse is ([ADR-0007][0007-vfs-file-state.md], [ADR-0008][0008-compiler-driver.md]) —
  or every consumer learns to re-discriminate a node the parser could not name.
- name resolution cannot anchor what the dot applies to,
  although the HIR it produces is where anchors live ([ADR-0005][0005-compiler-pipeline.md], [ADR-0010][0010-stable-entity-identity.md]):
  whether there is anything to anchor at all is decided by the receiver's type,
  which is an input of type checking and not of resolution.
- the first stage that could decide is type checking ([ADR-0005][0005-compiler-pipeline.md]),
  so no stage owns the construct,
  and a typo after the dot is reported by whichever stage happens to have the information
  rather than by the stage that owns the syntax.
- completion after a bare dot cannot know what to offer:
  the fields of the receiver's type, or the names and the paths an anchor may be written as.

This is the class of design the module system of [ADR-0004][0004-module-system.md] exists to rule out:
**language features (and errors) are syntax driven and not type driven** [against-query-based-compilers].
A construct whose meaning is decided by the output of later stages
forces every consumer to wait for those stages,
and makes an edit anywhere a possible change of meaning everywhere.

## Decision Drivers

- The shape of a construct must decide what it is and which stage owns it ([against-query-based-compilers]).
- A stage reads an input that is fixed before it runs ([ADR-0009][0009-pass-contract.md]);
  a stage that must know a type to classify a name is the global pass in disguise.
- A `.`-chain is the common shape of an expression,
  and every step of it must be anchored by what the text writes — a name or a path —
  and never by the type of what stands before the dot.
- An error must be reportable by the stage that owns the syntax:
  a mistake after `@`, after `.`, and after `::` are three different mistakes,
  found by three different stages.
- Completion must know at each operator what kind of thing it offers:
  a field, a name or a path, or an entity of a namespace.
- One character, one meaning:
  a sigil that carries two meanings puts the burden of telling them apart
  on the reader of the code and on the resolution of every occurrence.

## Considered Options

- **One `.` for all three uses** — fields, calls, and namespaces.
- **Two uses of `.`** — `value.field` and `value.function(args)`, with `::` for namespaces.
- **Three operators, `.` for calls** — `@` reads a field, `.` calls, `::` qualifies.
- **Three operators, `.` for fields** — the assignment that keeps the familiar meaning of `.`.

## Decision Outcome

Chosen option: "Three operators, `.` for calls",
because it is the only split in which every operator has exactly one meaning,
and because the operator that chains a value through a sequence of steps
is the one whose steps are found by the name or the path the text writes.

### A field is read with `@`: `receiver@field`

A field access is a receiver expression and a field name:

```mlk
fun port(config: Config): Int =
    config@server@port
```

The name is looked up in the type of the receiver,
which makes a field access a checking question:
the checker owns the construct,
and it is the one construct of the three
that cannot begin to be answered without the type of what stands before the sigil.

The receiver is any expression, and the result is a value.
A field access is postfix and chains left to right:
`config@server@port` is the field `port` of the field `server` of `config`,
and in `config@server.load()` the call applies to the field access before it.

### A call is written with `.`: `receiver.function(args)` means `function(receiver, args)`

The dot passes the receiver as the first argument of a call:
`receiver.function(arg₁, …, argₙ)` is `function(receiver, arg₁, …, argₙ)`,
where `function` is a name or a path.

```mlk
fun start(config: Config): State =
    config.load().validate().start()
```

Two properties make the operator resolvable where it is written:

- the callee is what the text writes:
  a name is what the module's scope resolves ([ADR-0010][0010-stable-entity-identity.md], [ADR-0011][0011-module-prelude.md]),
  and a path reaches a function that a plain name would not name —
  another module's, or one whose name the module means something else by.
- the callee is resolved where it is written, and never by the type of the receiver:
  name resolution anchors `load` and `validate` in the example above before any type is known,
  a name by the module's scope and a path by the tables it walks.

A path is what a step costs when a name is not enough:

```mlk
fun add(persons: Persons, name: Name, data: Data): Persons =
    persons.Map::insert(name, data)
```

`persons.Map::insert(name, data)` is `Map::insert(persons, name, data)`:
the path says which `insert` the step means,
and the words are the price of an anchor the text decides.

This is not method resolution in the sense of the languages around MLK:
the dot does not search a candidate set by the receiver's type,
and no impl decides what the callee is.
The dot substitutes the receiver into a call the text names,
and what the call means once its callee is known is the business of checking,
exactly as for `function(receiver, args)` written without a dot.

A dot is always written together with a call:
a dot and a callee with nothing after them are not an expression,
and the parser reports it.
The operator is a call with the receiver written before the callee,
and there is no spelling for a function with its first argument left open.

### A name in a namespace is reached with `::`: `project::data::utils`

A path is a name reached through the namespaces that hold it,
and `::` separates its segments:

```mlk
module project::app

use project::data::utils
use std::core::Int

fun handle(value: project::data::Handle): Int =
    project::data::start-app(1)
```

The root of a path does not change:
the first segment is a name, or the `project` keyword,
which is the project the module is written in ([ADR-0004][0004-module-system.md]).
A path is resolved by the tables of the project and not by the scope of the module:
every segment is an entity of a namespace,
and the tables that hold the entity are the project's, not the module's.
A path written after a dot is a path like any other:
what the dot adds is the receiver, and not another way to resolve a name.

Earlier records spell paths with the dot; the separator is what changes here,
not the decisions they record.

### Attributes are written `#[name]`

`@` reads a field now, so the attributes move off it:

```mlk
#[builtin]
type Int

#[no-prelude]
module project::prelude
```

An attribute is one name in brackets,
and a declaration may carry several, each in its own brackets.
Nothing else about attributes changes:
they are the same names, read by the same stages ([ADR-0010][0010-stable-entity-identity.md], [ADR-0011][0011-module-prelude.md]).

The brackets are also what a list of types is written with —
`Map[Int, String]` today, and in time the parameters of a generic declaration —
so the two lists can stand in one declaration:

```mlk
#[inline] fun id[T](x: T) -> T = x
```

They are told apart by the `#` and by the place:
an attribute list stands in front of a declaration,
and a list of types follows a name.
The coincidence is visual, and it is tolerated.
The attribute of [ADR-0011][0011-module-prelude.md] is spelled `#[no-prelude]` from now on;
that record's decision is untouched.

### What each stage owns after the split

| Written                   | Means                      | Owned by        | Reads                              |
| ------------------------- | -------------------------- | --------------- | ---------------------------------- |
| `receiver@field`          | a field of a type          | type checking   | the receiver's type                |
| `receiver.function(args)` | `function(receiver, args)` | name resolution | the scope, or the tables of a path |
| `path::segment`           | an entity of a namespace   | the global pass | the import and export tables       |

The parser classifies all three by shape alone,
and no stage after it has to reinterpret a construct of another.

### Decisions this record does not make

The pipeline operator `|>` is a later development, decided by [ADR-0013][0013-pipeline-operator.md]:
UFCS passes a value into the first argument,
and a value that belongs in another argument is written with a place for it.
Chains mix both operators:

```mlk
config
    .load()                     // load(config)
    |> merge(defaults, _)       // merge(defaults, load(config))
    .validate()                 // validate(merge(defaults, load(config)))
    |> write("out.toml", _)     // write("out.toml", validate(merge(...)))
```

Each callee in that chain is a name or a path the text writes,
anchored where it is written and never by a type —
a property the split is what makes possible.

The mechanism of syntactic sugar is a record of its own ([ADR-0014][0014-syntactic-sugar.md]):
where a spelling like `receiver.function(args)` stops being its own construct
and becomes the call it means,
and how diagnostics and source maps keep the spelling the reader wrote.

### Positive Consequences

- The parser gives every construct a node of its own by shape alone,
  and no stage after it reinterprets what it read ([ADR-0002][0002-lossless-syntax-tree.md]).
- Every call in a `.`-chain is anchored where it is written:
  in `config.load().validate()`, both `load` and `validate` are names of the module's scope,
  so what the chain is a chain of is known from the text alone,
  and no step of it waits for a type.
  A step whose callee is a path is anchored by the entities the path names,
  which is the wait every path has ([ADR-0005][0005-compiler-pipeline.md]).
- Each error is owned by one stage:
  the parser reports a dot that is not followed by a call,
  local resolution reports a name that is not in scope,
  the checker reports a field the type does not have,
  and the global pass reports an entity a namespace does not hold.
- Completion after an operator has one kind of candidate:
  the fields of a type after `@`,
  the names of the module's scope after `.`,
  the entities of a namespace after `::`.
- An edit in another module changes what this module's text is checked against
  — which is what an interface is for ([ADR-0008][0008-compiler-driver.md], [ADR-0010][0010-stable-entity-identity.md]) —
  but not what its constructs are:
  a field access stays a field access, a call stays a call, and a path stays a path.

### Negative Consequences

- Every path in every module changes its separator,
  so every module, spec, and snapshot of the repository is rewritten.
  The change is mechanical, and it touches a great many files at once.
- The attribute syntax changes with it:
  the grammar, the parser's lookahead before a declaration, and lowering.
  Every attribute in the standard library and in the specs is respelled.
- A two-segment path with a call — `data.start-app(1)` — parses as a call of `start-app`
  with `data` as its first argument,
  so old text of that shape does not fail to parse:
  where a value of the same name is in scope, it silently means something else.
- `.` is not a way to read a field, and `value.function` is not a way to name a function:
  there is no method reference and no `value.field` fallback,
  so a reader who knows other languages meets an error where the habit writes `.`.
- `@` has no precedent as a field sigil:
  in other languages the character marks attributes and annotations,
  and every reader learns `data@x` once.
- A callee that a plain name does not name costs a path, and a path is long:
  `persons.Map::insert(name, data)` is more words than a language
  that finds a method by the type of its receiver would need;
  the words are what an anchor the text decides costs.
- An attribute list and a list of types share the brackets,
  so a declaration that carries both takes a second look.

## Pros and Cons of the Options

### One `.` for all three uses

Fields, calls, and namespaces on one character, as Rust and its relatives do.

- Good, because the surface stays small,
  and the character is the one every reader knows.
- Bad, because the meaning of `value.name` is decided by types and by global tables,
  not by the text, so no stage can own the construct:
  the parser cannot choose a node,
  local resolution cannot anchor a name,
  and the first stage that could decide is the checker.
- Bad, because the IDE can say nothing about a dot —
  neither the candidates nor the error —
  before resolution and checking have run over the project.
- Bad, because an edit to a foreign type can change what a construct here denotes,
  not only whether it is correct.

### Two uses of `.`

`value.field` and `value.function(args)`, with `::` for namespaces.

- Good, because it is the assignment most readers know from the languages around MLK.
- Good, because the part of the overload that reaches the whole project
  is the part that is split off.
- Bad, because the character keeps two meanings,
  told apart by what follows it: a name is a field, a name with a call is a call.
  Every rule, every diagnostic, and every completion needs both.
- Bad, because completion after a bare dot cannot know which list to build,
  the fields of the receiver's type or the names in scope, and must offer a union.
- Bad for a language with first-class functions:
  a call of a field and a call of a function whose first argument is the receiver
  would differ by a pair of parentheses — `(value.field)()` against `value.field()` —
  and would mean entirely different things.

### Three operators, `.` for calls

Chosen. Described above.

- Good, because every operator has one meaning and one stage that owns it.
- Good, because the chaining operator is the one whose steps are anchored by the text —
  a name the scope holds, or a path the tables hold —
  so a chain of calls waits for no type.
- Good, because the operation that cannot begin without a type — a field read —
  is marked with a sigil of its own,
  and a reader sees which steps of an expression need types and which need only what the text writes.
- Bad, because the language has three sigils where it had one,
  and one of them, `@`, is new as a field access.
- Bad, because every path and every attribute is respelled.

### Three operators, `.` for fields

`.` reads a field, `::` qualifies a namespace,
and the spare sigil, `@`, is the call: `value.x` and `value@f(args)`.

- Good, because `.field` keeps the meaning it has in almost every language,
  so the most familiar spelling stays on the most familiar operation.
- Bad, because it puts the one operation that cannot be found without a type
  on the operator that chains:
  a chain would be a chain of field reads, `a.x.y`,
  and every call in it would leave the dot for an unfamiliar sigil.
- Bad, because the language is built around passing a value through a sequence of functions,
  and that shape would be the one spelled with a sigil no reader has seen,
  while a selection from a value would keep the dot.

## Links

- The principle: Against Query Based Compilers: <https://matklad.github.io/2026/02/25/against-query-based-compilers.html>
- Module rules and incrementality: [0004-module-system.md]
- The pipeline and its stages: [0005-compiler-pipeline.md]
- The operator a chain gets for the other arguments: [0013-pipeline-operator.md]
- What a spelling is, and where it stops: [0014-syntactic-sugar.md]
- Static tree structure per node kind: [0002-lossless-syntax-tree.md]
- What a pass may read: [0009-pass-contract.md]
- Entities, names, and anchors: [0010-stable-entity-identity.md]
- The prelude's names in scope: [0011-module-prelude.md]
- Implementation: [mlkc-parser], [mlkc-lower], [mlkc-hir-def]

[0002-lossless-syntax-tree.md]: 0002-lossless-syntax-tree.md
[0004-module-system.md]: 0004-module-system.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0007-vfs-file-state.md]: 0007-vfs-file-state.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0010-stable-entity-identity.md]: 0010-stable-entity-identity.md
[0011-module-prelude.md]: 0011-module-prelude.md
[0013-pipeline-operator.md]: 0013-pipeline-operator.md
[0014-syntactic-sugar.md]: 0014-syntactic-sugar.md
[against-query-based-compilers]: https://matklad.github.io/2026/02/25/against-query-based-compilers.html
[mlkc-parser]: ../../crates/mlkc-parser
[mlkc-lower]: ../../crates/mlkc-lower
[mlkc-hir-def]: ../../crates/mlkc-hir-def
