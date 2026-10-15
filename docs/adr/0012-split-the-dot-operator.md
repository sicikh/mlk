# Spell a field with `.`, a uniform call with `->`, and a namespace with `::`

- Status: accepted
- Date: 2026-10-16

## Context and Problem Statement

Three things a reader writes after a value look alike,
and each of them is resolved by a stage of its own:

- `receiver.field` reads a field,
  and the field is looked up in the type of the receiver:
  a question type checking answers and name resolution cannot ([ADR-0005][0005-compiler-pipeline.md]);
- `receiver->function(args)` calls a function with the receiver as its first argument,
  where the callee is a name the module's scope holds or a path the tables of the project hold:
  a question name resolution answers before any type is known
  ([ADR-0010][0010-stable-entity-identity.md], [ADR-0011][0011-module-prelude.md]);
- `path::segment` names an entity of a namespace,
  which the import and export tables of the project hold ([ADR-0004][0004-module-system.md]).

One character for all three would make the meaning of `value.name`
a question about types and global tables rather than about the text,
with a cost at every stage:

- the parser must commit to a node kind at parse time ([ADR-0002][0002-lossless-syntax-tree.md]),
  and no kind fits: a path segment, a field access, and a call with an implicit argument
  have different children.
  Either the tree keeps one shape and is rewritten once the world is known ---
  and then it is no longer a function of the text,
  which is what the identity of a parse is ([ADR-0007][0007-vfs-file-state.md], [ADR-0008][0008-compiler-driver.md]) ---
  or every consumer learns to re-discriminate a node the parser could not name.
- name resolution cannot anchor what the character applies to,
  although the HIR it produces is where anchors live ([ADR-0010][0010-stable-entity-identity.md]):
  whether there is anything to anchor at all is decided by the receiver's type,
  which is an input of type checking and not of resolution.
- the first stage that could decide is type checking ([ADR-0005][0005-compiler-pipeline.md]),
  so no stage owns the construct,
  and a typo after the character is reported by whichever stage happens to have the information
  rather than by the stage that owns the syntax.
- completion after a bare character cannot know what to offer:
  the fields of the receiver's type, or the names and the paths an anchor may be written as.

This is the class of design the module system of [ADR-0004][0004-module-system.md] exists to rule out:
**language features (and errors) are syntax driven and not type driven** ([against-query-based-compilers]).
A construct whose meaning is decided by the output of later stages
forces every consumer to wait for those stages,
and makes an edit anywhere a possible change of meaning everywhere.

The language splits the three, one construct and one sigil each,
and this record decides the spelling of each.
It found two of the spellings the wrong way round.

**The field got the sigil nobody knows.**
`.field` is the spelling of a selection in the languages around MLK,
and `@` is not a field sigil anywhere:
in other languages the character marks attributes and annotations,
and every reader learns `data@x` once.
The operation a reader reaches for first should not be the one that has to be learned.

**The call got the spelling of a selection.**
`receiver.function(args)` is how the languages around MLK spell a method call,
and a method call it is not:
the callee is a name of the module's scope --- or a path ---
and the receiver is only its first argument.
The spelling promises a search in the receiver's type, and no search happens.
A reader who knows those languages has to unlearn what a dot means
at the one place where MLK borrows their shape.

**The arrow spells the step, and the dot spells the selection.**
What a chain of calls is made of is not a selection but a flow:
a value goes into a call, and what the call produced goes into the next one.
`x->f(args)` reads as exactly that --- send `x` into `f` ---
and the arrow has a precedent of its own:
`->` is the threading arrow of Clojure,
and `p->f(args)` of the C family is a call written after the value it applies to.
The dot is left where every reader expects the selection.

**The spellings this record replaces.**
They stood here as the other way round:
a field was read with `@` (`receiver@field`),
and a call was written `receiver.function(args)`.
This note is all that remains of them,
and no record, plan, or code is read against the old spellings.

The question: which sigil reads a field,
and which sigil calls a function with the value as its first argument?

## Decision Drivers

- The shape of a construct must decide what it is and which stage owns it ([against-query-based-compilers]).
- A stage reads an input that is fixed before it runs ([ADR-0009][0009-pass-contract.md]);
  a stage that must know a type to classify a name is the global pass in disguise.
- One character, one meaning:
  the character decides what a construct is, and no reader and no rule has to hold two meanings apart.
- The familiar meaning of a `.` --- the name after it belongs to the value before it ---
  belongs on the operation that is a selection from a value.
- The chaining operator is marked as a call, and every call in a chain is anchored
  by what the text writes --- a name, or a path the tables hold ---
  and never by the type of the receiver
  ([ADR-0010][0010-stable-entity-identity.md], [ADR-0011][0011-module-prelude.md], [against-query-based-compilers]).
- The operators that pass a value through calls read as one family:
  `->` sends the value as the first argument, `|>` sends it where the `_` is written
  ([ADR-0013][0013-pipeline-operator.md]).
- The surface stays typeable:
  a host reads `x->f(args)` token by token while it is written,
  and the `-` of that prefix must not become part of a name ([ADR-0029][0029-dash-in-names.md]).
- Precedent: `x.field` is a selection in every language MLK's readers know,
  `p->f(args)` is a call applied to the value, and `=>` is the arrow of a lambda
  in most of the languages around MLK.

## Considered Options

- **The spellings that stood here before** ---
  `@` reads a field, `.` calls, `::` qualifies.
- **Two uses of `.`** --- `value.field` and `value.function(args)`, with `::` for namespaces.
- **Three operators, `.` for fields and `->` for calls** ---
  `x.field` reads a field, `x->f(args)` means `f(x, args)`, `::` qualifies.
- **Three operators, `.` for fields and `@` for calls** ---
  the same split with the call on `@`.
- **One `.` for all three uses** --- fields, calls, and namespaces.

## Decision Outcome

Chosen option: "Three operators, `.` for fields and `->` for calls",
because it is the split in which every operation is found where a reader looks for it:
the selection is on the dot every language spells a selection with,
the call is on an arrow that marks a call and a flow,
and the two operators a chain is made of --- `->` and `|>` --- are one family of arrows.

### A field is read with `.`: `receiver.field`

A field access is a receiver expression and a field name:

```mlk
fun port(config: Config): Int =
    config.server.port
```

The name is looked up in the type of the receiver,
which makes a field access a checking question:
the checker owns the construct,
and it is the one construct of the three
that cannot begin to be answered without the type of what stands before the dot.

The receiver is any expression, the result is a value, and a field access is postfix:
`config.server.port` is the field `port` of the field `server` of `config`,
and in `config->load().state` the dot reads a field of what the step produced.
A path is read as a path before the dot, so `project::data.field` reads
the field of the entity the path denotes.

A field is a value, and so is what it holds:

```mlk
fun arrive(config: Config, request: Request): Response =
    config.handler(request)
```

`config.handler` names what the field holds, and the call written after it
calls that function with `request`.
The sigil says which of the two is written:
a field read is the dot, and the call of what it holds is the argument list after it,
while the call that passes the receiver first is the arrow.

The dot is also where a host completes the fields of the receiver's type.
That list is a type question, and it is a question a reader expects the dot to ask:
every dot-completion a reader of the languages around MLK has used
offers the members of what stands before it.

### A uniform call is written with `->`: `receiver->function(args)`

The arrow passes the receiver as the first argument of a call:
`receiver->function(arg₁, …, argₙ)` is `function(receiver, arg₁, …, argₙ)`,
where `function` is a name or a path.

```mlk
fun start(config: Config): State =
    config->load()->validate()->start()
```

Two properties make the operator resolvable where it is written:

- the callee is what the text writes:
  a name is what the module's scope resolves ([ADR-0010][0010-stable-entity-identity.md], [ADR-0011][0011-module-prelude.md]),
  and a path reaches a function that a plain name would not name ---
  another module's, or one whose name the module means something else by;
- the callee is resolved where it is written, and never by the type of the receiver:
  name resolution anchors `load`, `validate`, and `start` in the example above
  before any type is known,
  a name by the module's scope and a path by the tables it walks.

A path is what a step costs when a name is not enough:

```mlk
fun add(persons: Persons, name: Name, data: Data): Persons =
    persons->Map::insert(name, data)
```

`persons->Map::insert(name, data)` is `Map::insert(persons, name, data)`:
the path says which `insert` the step means,
and the words are the price of an anchor the text decides.

This is not method resolution in the sense of the languages around MLK:
the arrow does not search a candidate set by the receiver's type,
and no impl decides what the callee is.
The arrow substitutes the receiver into a call the text names,
and what the call means once its callee is known is the business of checking,
exactly as for `function(receiver, args)` written without an arrow.

An arrow is always written together with a call:
`value->` and `value->function` are not expressions,
and the parser reports it.
The operator is a call with the receiver written before the callee,
and there is still no spelling for a function with its first argument left open.

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
A path written after an arrow is a path like any other:
what the arrow adds is the receiver, and not another way to resolve a name.

### Attributes keep `#[name]`, and `@` leaves the language

```mlk
#[builtin]
data Int

#[no-prelude]
module project::prelude
```

The spellings that stood here before moved attributes off `@` because `@` read a field.
The reason is gone, and the spelling stays:
`#[name]` has a precedent of its own, the stages that read an attribute are untouched,
and re-spelling every attribute of every module would buy nothing this record needs.
The decision of [ADR-0011][0011-module-prelude.md] about `#[no-prelude]` is untouched with it.

`@` no longer spells anything:
a `@` in source is an error of the lexer,
and the character is free if a later decision wants it.

### The chain, and the pipe

The steps of a chain are the arrow calls, and [ADR-0013][0013-pipeline-operator.md] is untouched:
when the value belongs in an argument that is not the first,
the pipe writes a place for it among the arguments:

```mlk
config
    ->load()                    // load(config)
    |> merge(defaults, _)       // merge(defaults, load(config))
    ->validate()                // validate(merge(defaults, load(config)))
    |> write("out.toml", _)     // write("out.toml", validate(merge(...)))
```

What stands after a step is written after the value the pipeline is,
so the `->validate()` of the third line applies to what the pipe produced.
A step written on the value may use the arrow as well:
`input |> map->insert("key", _)` is a step of `insert`,
with `map` and `"key"` before the value,
which the spellings that stood here before wrote `input |> map.insert("key", _)`.

The two operators read as one family:
`->` sends the value as the first argument,
and `|>` sends it where the `_` is written.
Both are arrows, and both pass a value into a call the text names.

### The lambda arrow becomes `=>`

```mlk
fun added(value: Int): Int =
    fn(x) => x + value
```

`->` is the arrow of a step now, and a second meaning for it
is the class of design this record and the split exist to remove.
The lambda takes the arrow the languages around MLK write a lambda with,
and the rest of the rule does not change:
the parameters between the parentheses, the body an expression, `fn() => 1`.

### What each stage owns

| Written                    | Means                      | Owned by        | Reads                              |
| -------------------------- | -------------------------- | --------------- | ---------------------------------- |
| `receiver.field`           | a field of a type          | type checking   | the receiver's type                |
| `receiver->function(args)` | `function(receiver, args)` | name resolution | the scope, or the tables of a path |
| `path::segment`            | an entity of a namespace   | the global pass | the import and export tables       |

The parser classifies all three by shape alone,
and no stage after it has to reinterpret a construct of another.

### What this record does not decide

- How a number with a fractional part will be written beside a field read:
  the language has no float literal today,
  and it is the dot of a field that such a rule has to sit beside.
- Whether a linter exists that points `x |> f(_)` at `x->f()`:
  the arrow is the spelling to prefer, and the rule that says so is a linter's, not the compiler's.

### Positive Consequences

- `.` means what it means in the languages around MLK:
  a field read is spelled the way every reader expects to read a selection,
  and no reader has to learn `@` first.
- The shapes a chain is made of are told apart by their sigils:
  `x.field` is a value, `x.field(args)` calls what the field holds,
  and `x->function(args)` is the uniform call.
  The pair of parentheses is no longer what decides between a call of a field
  and a call with the receiver first.
- Every arrow call is anchored where it is written,
  so a chain of steps waits for no type ---
  the property the split exists for,
  carried by the operator whose shape is a call.
- A function a value holds can be named and called:
  the dot names it, and the argument list calls it.
- Completion after `->` is text-only --- the names of the module and the paths ---
  and completion after `.` is the fields of a type, which is where a type question belongs.
- Each error is owned by one stage:
  the parser reports an arrow or a dot that is not followed by what belongs there,
  local resolution reports a name that is not in scope,
  the checker reports a field the type does not have,
  and the global pass reports an entity a namespace does not hold.
- The two arrows read as one family, and a chain of steps reads in the direction the value flows.
- The attributes keep the spelling they have, so the standard library and the specs
  do not have to be touched for them.

### Negative Consequences

- Every step of every chain is respelled, and so is every lambda:
  the change is mechanical, and it touches every module, spec, and snapshot of the repository.
- `->` resembles the member call of the C family while it is not one:
  the callee is a free name, found by the text, and a reader may still look for a method.
- The dot is now the type-dependent operator:
  a host can offer the fields of a receiver only once its type is known,
  and a chain of field reads is a chain that waits for types.
  What a reader gets in exchange is that the type-dependent sigil
  is not the one the language chains with.
- `@` leaves the language:
  a reader who learned the previous spelling gets a diagnostic where a field read used to be.
- The lambda arrow and the step arrow differ by one character, `=>` against `->`,
  and a reader has to keep them apart.

## Pros and Cons of the Options

### The spellings that stood here before

`@` reads a field, `.` calls, `::` qualifies.

- Good, because every operator has one meaning and one stage that owns it.
- Good, because the chaining operator is anchored by the text,
  so a chain of calls waits for no type.
- Bad, because the selection from a value --- the operation every reader reaches for first ---
  is spelled with a sigil no language uses for it, and every reader learns `@` once.
- Bad, because a call that is not a member is spelled like a member call,
  and the reader is promised a search that never happens.
- Bad, because a dot is never a value:
  `value.field` is an error, so the operation every reader spells with a dot
  has no dot at all.

### Two uses of `.`

`value.field` and `value.function(args)`, with `::` for namespaces.

- Good, because it is the assignment most readers know from the languages around MLK.
- Bad, because the character keeps two meanings told apart by what follows it,
  and every rule, diagnostic, and completion needs both.
- Bad, because what `value.name` is --- a field read or a call ---
  is decided by the parenthesis that follows the name and not by the sigil.
- Bad, because completion after a bare dot must offer a union of the fields and the names in scope.

### Three operators, `.` for fields and `->` for calls

Chosen. Described above.

- Good, because each operation is spelled where a reader looks for it:
  the selection on the dot, the call on an arrow that marks a call and a flow.
- Good, because the sigil, not a pair of parentheses, tells a field from a step:
  `x.field`, `x.field(args)`, and `x->function(args)` are three shapes with three meanings.
- Good, because the arrow is anchored by the text and not by the receiver's type,
  so a chain of steps still waits for no type.
- Good, because the two arrows of the language --- `->` and `|>` --- are one family of flows.
- Good, because a field is a value, so what a value holds can be named, passed, and called.
- Bad, because two sigils sit where a reader of other languages expects one,
  and which of them a chain is written with has to be learned.
- Bad, because every chain and every lambda of every module is respelled.

### Three operators, `.` for fields and `@` for calls

`.` reads a field, `::` qualifies a namespace, and `@` calls: `value.x` and `value@f(args)`.

- Good, because `.field` keeps the meaning it has in almost every language.
- Bad, because the call --- the shape a chain is made of, and the common shape of the language ---
  is the one that would carry the sigil no reader has seen as a call.
- Bad, because a chain of steps would be a chain of `@`, `x@f(a)@g(b)`,
  which neither reads as a call nor marks a flow.
- Bad, because a reader would have to learn two unfamiliar things at once:
  what the sigil means, and that chains are written with it.

### One `.` for all three uses

Fields, calls, and namespaces on one character, as Rust and its relatives do.

- Good, because the surface stays small, and the character is the one every reader knows.
- Bad, because the meaning of `value.name` is decided by types and by global tables,
  not by the text, so no stage can own the construct.
- Bad, because the IDE can say nothing about a dot ---
  neither the candidates nor the error ---
  before resolution and checking have run over the project.

## Links

- The other operator a chain is written with: [0013-pipeline-operator.md]
- What a spelling is, and where it stops: [0014-syntactic-sugar.md]
- How a `-` is read while `x->f(args)` is typed: [0029-dash-in-names.md]
- Module rules and incrementality: [0004-module-system.md]
- The pipeline and its stages: [0005-compiler-pipeline.md]
- What a pass may read: [0009-pass-contract.md]
- Entities, names, and anchors: [0010-stable-entity-identity.md]
- The prelude's names in scope: [0011-module-prelude.md]
- Static tree structure per node kind: [0002-lossless-syntax-tree.md]
- The principle: Against Query Based Compilers: <https://matklad.github.io/2026/02/25/against-query-based-compilers.html>
- Implementation: [mlkc-syntax], [mlkc-parser], [mlkc-lower], and the editor's [grammar]

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
[0029-dash-in-names.md]: 0029-dash-in-names.md
[against-query-based-compilers]: https://matklad.github.io/2026/02/25/against-query-based-compilers.html
[mlkc-syntax]: ../../crates/mlkc-syntax
[mlkc-parser]: ../../crates/mlkc-parser
[mlkc-lower]: ../../crates/mlkc-lower
[grammar]: ../../web/src/lib/grammar/mlk.grammar
