# Add the pipeline operator `|>`

- Status: accepted
- Date: 2026-09-27

## Context and Problem Statement

Uniform function call syntax passes a value into the first argument of a call ([ADR-0012][0012-split-the-dot-operator.md]),
and a chain of such calls is the common shape of an expression:

```mlk
config.load().validate().start()
```

The dot is the spelling the language wants for a step.
It is what a reader of almost any language already reads as a step,
it is what says where the value goes by itself,
and it is what a host completes after a value: the names of the module are the candidates.

Some steps want the value in another argument.
Merging a config into defaults is `merge(defaults, config)`,
and the dot cannot write it: the dot always puts the value first.
Written as calls, the step leaves the order the value flows in:

```mlk
write("out.toml", validate(merge(defaults, load(config))))
```

The calls are read from the outside in, and the value is read at the innermost place.

The operator the languages MLK borrows from use for this is the pipe:
a value, an arrow, and the call the value goes into,
with a written place for the value:

```mlk
a + b
|> make-data(_)
.prepare()
|> map.insert("key", _)
```

The question: what stands on the right of `|>`, and what does the operator mean?

## Decision Drivers

- A chain reads in the order the value flows: the left side is what runs first.
- A step is a call, and a call is anchored where it is written ([ADR-0012][0012-split-the-dot-operator.md]).
- The pipe is the fallback of the dot, not its rival:
  the dot is the spelling of a step wherever it can write it,
  and `|>` is for the steps the dot cannot write.
- The value is used once, however many places it is written in.
- What the operator does is decided by its shape and never by a type
  ([ADR-0012][0012-split-the-dot-operator.md], [against-query-based-compilers]).
- What it means is a construct the language writes directly ([0014-syntactic-sugar.md]).

## Considered Options

- **No pipeline** — a chain is written with the dot and with plain calls.
- **A pipeline that always passes the first argument** — `x |> f(a)` is `f(x, a)`.
- **A pipeline with a place for the value among the arguments** — `x |> f(a, _)` is `let v = x in f(a, v)`.
- **A pipeline whose right side is an expression with places in it** — `x |> g(_ + 1)` is `let v = x in g(v + 1)`.

## Decision Outcome

Chosen option: "A pipeline with a place for the value among the arguments",
because it is the only one where the value is written where it goes,
and where a step stays a call.

### The meaning

```mlk
x |> f(_)                   <=> let v = x in f(v)
x |> f(1, _)                <=> let v = x in f(1, v)
data |> map.insert("k", _)  <=> let v = data in insert(map, "k", v)
expr |> f(_, _)             <=> let v = expr in f(v, v)
```

The left side is bound before the call, and every `_` is the binding:
the value is evaluated once, before the call and before its other arguments.
The left side is therefore what runs first, whatever the right side writes,
which is what keeps a chain a flow of values rather than a tree of calls.

### A step is a call with a place

The right side of `|>` is a call with at least one `_` among its arguments:

- a call with no `_` has nowhere to put the value, and the parser says so:
  `x |> f(a)` is not a step;
- a name or a path alone is not a step either:
  `x |> f` is not written, because the step it means — `f(x)` —
  is what the dot writes as `x.f()`,
  and it is the dot that a host completes after a value.

`x |> f(_)` is a step, and it is the step `x.f()` in other words:
the dot is the spelling to prefer, and a linter may later say so.

A place is a leaf of the expression grammar,
and the parser reads a `_` only among the arguments of the call on the right of a `|>`;
anywhere else in an expression a `_` is not a value, and the parser reports it.

### What stands on the right

A call: a callee, and an argument list with a place among the arguments.

The callee is a name the module's scope holds ([ADR-0012][0012-split-the-dot-operator.md], [ADR-0011][0011-module-prelude.md]),
or a path,
and a call written with the dot is a call too:
`data |> map.insert("key", _)` is a step of `insert`,
with `map` and `"key"` before the value.
The receiver of a step's dot call is a name or a path,
so a step is one call:
a call on the result of another call goes inside the arguments instead
(`x |> g(f(1), _)`).

A step ends with the argument list of its call.
What is written after it — a dot, another `|>`, a binary operator — is written after the pipeline:
`.prepare()` after a step is the dot called on the value the step produced.

### The shape

`|>` is left-associative: `x |> f(_) |> g(_)` is `g(f(x))`.
It is looser than every binary operator (`a + b |> f(_)` is the step of `f` on `a + b`)
and tighter than a sequence ([0014-syntactic-sugar.md]).

### What the operator costs

The operator is a spelling ([0014-syntactic-sugar.md]):
it means a binding and a call, and it adds no construct to the HIR.
Every pass after lowering reads a `let` and a call.

### Diagnostics

- What is wrong with the spelling as syntax is the parser's:
  a `_` where a pipeline does not take a value,
  a call on the right with no `_`,
  a right side that is not a call.
- What is wrong with its meaning is the call's own:
  the number and the types of the values the call passes, the value among them.
  A diagnostic points at the pieces the reader wrote,
  and the map is what says which piece each value is ([0014-syntactic-sugar.md]).

### A host reads a pipeline as a binding and a call

The binding is a node the lowering makes up, under a name no module can write,
and it is about the value it binds, which is written on the left.
Every `_` is a reference to the binding, and a reference reads as the `_` it stands for.
The call is about the whole step, and the left side is the expression the binding is bound to.

## Positive Consequences

- The left side runs first, and a chain reads in one direction.
- A step is a call, so a step is anchored where it is written,
  by the same rules as any other call ([ADR-0012][0012-split-the-dot-operator.md]).
- The value reaches any argument of any call form without leaving the chain:
  the dot covers the first argument, and the pipe covers the rest.
- `x |> f(_)` and `x.f()` mean the same step:
  the dot is the shorter spelling, and a linter may point a pipe at it.
- The operator adds no construct:
  the passes see a `let` and a call.

## Negative Consequences

- The language has two chaining operators;
  the pipe is for the steps the dot cannot write, and a reader has to know both.
- A `_` is a leaf of the expression grammar that is valid in one place only,
  and the parser reports it everywhere else.
- `x |> f(a)` looks like a step and is a mistake: a step needs a place.
- Every step is a `let` in the HIR that no reader wrote:
  a reader of a dump sees a binding that is not in the text.
- The receiver of a step's dot call is a name or a path,
  so a step is one call:
  a call on the result of another call goes inside the arguments instead of before the dot.

## Pros and Cons of the Options

### No pipeline

- Good, because the language keeps one chaining operator, and it is the one every reader knows.
- Bad, because a step that is not the first argument must leave the chain:
  `write("out.toml", validate(merge(defaults, load(config))))` reads from the innermost place out.
- Bad, because the value is named and threaded by hand wherever it goes into the middle.

### A pipeline that always passes the first argument

`x |> f(a)` is `f(x, a)`, the thread-first of the Lisp family.

- Good, because the operator is as simple as the dot, and no place syntax is needed.
- Bad, because it fixes which argument the value takes,
  which is the one thing the dot already fixes;
  a step that wants the value second has no spelling.
- Bad, because `f(a)` written on the right means something other than `f(a)` written elsewhere,
  which is a reading a reader has to hold apart.

### A pipeline with a place for the value among the arguments

Chosen. Described above.

- Good, because the value is written where it goes.
- Good, because the left side is bound first: a chain is a flow of values, not a tree of calls.
- Good, because a place is explicit:
  the call says where the value goes, and no rule about types is needed.
- Bad, because the expression grammar gains a placeholder, valid in one place.

### A pipeline whose right side is an expression with places in it

`x |> g(_ + 1)` is `g(x + 1)` — a place is a leaf of an expression, not an argument of a call.

- Good, because it composes: a place may stand anywhere a value may.
- Bad, because a step is no longer a call:
  `x |> g(_ + 1)` applies `g` to a sum,
  and the reader has to read the step apart to see the call in it.
- Bad, because what a place may stand in, and how many of them a step may write,
  is a rule about the shape of the right side, larger than the operator it serves.

## What this record does not decide

- The mechanism that lowers the operator: [0014-syntactic-sugar.md].
- A place anywhere other than the arguments of the step's call:
  that is the fourth option, and it is not admitted here.
- Whether a linter exists that points `x |> f(_)` at `x.f()`:
  the dot is the spelling to prefer, and the rule that says so is a linter's, not the compiler's.

## Links

- The other chaining operator, and the rules of a step: [0012-split-the-dot-operator.md]
- The mechanism every spelling is lowered by: [0014-syntactic-sugar.md]
- Names the module's scope holds: [0011-module-prelude.md]
- The principle: Against Query Based Compilers: <https://matklad.github.io/2026/02/25/against-query-based-compilers.html>
- Implementation: [mlkc-parser], [mlkc-lower]

[0011-module-prelude.md]: 0011-module-prelude.md
[0012-split-the-dot-operator.md]: 0012-split-the-dot-operator.md
[0014-syntactic-sugar.md]: 0014-syntactic-sugar.md
[against-query-based-compilers]: https://matklad.github.io/2026/02/25/against-query-based-compilers.html
[mlkc-parser]: ../../crates/mlkc-parser
[mlkc-lower]: ../../crates/mlkc-lower
