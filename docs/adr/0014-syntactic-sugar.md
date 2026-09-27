# Make lowering the only stage that knows syntactic sugar

- Status: accepted
- Date: 2026-09-27

## Context and Problem Statement

The language says more than one thing for the same construct, and that is deliberate:
`(e)` is `e`,
`receiver.function(args)` is `function(receiver, args)` ([ADR-0012][0012-split-the-dot-operator.md]),
the sequence `e1; e2` is `let _ = e1 in e2`,
and the pipeline `x |> f(a, _)` is `let v = x in f(a, v)` ([ADR-0013][0013-pipeline-operator.md]).
A form of this kind — a _spelling_ of a construct —
is worth having when it says in a shorter or a more familiar way
what the language already says,
and it costs nothing to a reader who does not use it.

It does cost the compiler something, and the question is where that cost is paid:
somewhere between the text and the meaning,
a spelling has to stop being itself and become the construct it means.

Two pressures meet at that point.

The stages after the reading want one construct per meaning.
A spelling that survives into the HIR is a construct of its own:
the resolver, the checker, and the codegen each learn one more case per spelling,
and each of those cases has to agree with the others forever.
A language with four spellings of one construct is a language with four places to fix one rule.

A reader and a host want the spelling.
A diagnostic points at a piece of text,
and a language server maps positions in the text to what they mean;
a compiler that has forgotten the spelling by the time it reports a mistake
has to reconstruct it,
and one that cannot reconstruct it reports about something the reader did not write.

MLK already has a stage whose business this is.
Lowering reads syntax and produces the HIR,
and it already reads `(e)` as `e`,
keeping the parentheses as the range of the node it makes.
The question:
is lowering where every spelling stops,
and what keeps the written form readable to everything that comes after?

## Decision Drivers

- Diagnostics must not get worse:
  a mistake is reported about a piece of what the reader wrote, and points at it ([ADR-0012][0012-split-the-dot-operator.md]).
- A construct is implemented once:
  the resolver, the checker, and the codegen read the meaning, not the spellings.
- A host must be able to map a position in the text to the node of the HIR that the text is,
  and back ([ADR-0008][0008-compiler-driver.md], [ADR-0009][0009-pass-contract.md]).
- The parse stays a function of the text:
  a spelling is syntax, and the tree keeps it as it is written ([ADR-0002][0002-lossless-syntax-tree.md], [ADR-0007][0007-vfs-file-state.md]).
- The HIR stays the meaning:
  what crosses a module boundary is an interface over the HIR ([ADR-0004][0004-module-system.md], [ADR-0010][0010-stable-entity-identity.md]),
  and a spelling is not something another module can be told about.

## Considered Options

- **Every stage knows every spelling** — the HIR keeps a variant per spelling, and each pass handles each.
- **The parser rewrites the spelling** — the tree holds the meaning, and the spelling is trivia.
- **A rewrite pass of its own** — a surface HIR is lowered, and a pass after it rewrites the surface into a core.
- **Declare the rules once, and generate the code from them** — a table of rewrites, and the lowering follows from it.
- **Lowering is the boundary** — the HIR holds the meaning; the written form stays in the source map.

## Decision Outcome

Chosen option: "Lowering is the boundary",
because it is the only one of the five
where neither the parse nor any pass after lowering pays for a spelling,
and where the written form is kept by the one thing a host already reads positions from.

### A spelling stops at lowering, and what it means is a construct the language writes directly

A spelling is admitted only when the language can also write what it means.
The spellings the language has decided:

```mlk
(e)                   lowers to  e
receiver.function(a)  lowers to  function(receiver, a)
e1; e2                lowers to  let _ = e1 in e2
x |> f(a, _)          lowers to  let v = x in f(a, v)
x |> f(_, _)          lowers to  let v = x in f(v, v)
```

This is what keeps the passes from growing.
A spelling never makes a construct the language does not already have,
so no pass learns a case for one:
a sequence is a `let` with a wildcard pattern, and a `let` with a wildcard is written;
a dot call is a call, and a call is written;
a pipeline is a `let` and a call, and both are written.

The HIR grows no variant for a spelling.
The one it has today for a sequence, `Expr::Seq`, is not needed:
there is nothing a sequence means that a `let` with a wildcard does not,
and a pass that reads a `let` needs no second case for it.

### The spelling of a sequence: `e1; e2` means `let _ = e1 in e2`

```mlk
e1; e2        lowers to  let _ = e1 in e2
e1; e2; e3    lowers to  let _ = e1 in let _ = e2 in e3
```

The operator nests to the right, and it is the loosest operator of an expression —
looser than the pipeline ([ADR-0013][0013-pipeline-operator.md]) —
so `a; b + c; d` is the sequence of `a`, `b + c`, and `d`.

The `;` and the expressions are written;
the wildcard pattern is what the lowering makes up from nothing,
and a node made up from nothing has no range.

### The spelling of a call: `receiver.function(args)` means `function(receiver, args)`

Decided by [ADR-0012][0012-split-the-dot-operator.md], and concrete here:

- the callee is what is written after the dot — a name or a path ([ADR-0012][0012-split-the-dot-operator.md]) —
  and its range is that piece of the text;
- the receiver is the first argument, and its range is the receiver as written;
- the arguments are read in the order they are written, after the receiver;
- the call is about the whole spelling, as a parenthesized expression is about the parentheses.

Nothing here is made up: every node of the call is a piece the reader wrote.

### The spelling of a pipeline: `x |> f(a, _)` means `let v = x in f(a, v)`

Decided by [ADR-0013][0013-pipeline-operator.md], and concrete here:

- the step is a `let` the lowering makes up, and the call the step is;
- the call is read as any call is:
  a step written with the dot is read as the call it means ([ADR-0012][0012-split-the-dot-operator.md]),
  so the callee of `data |> map.insert("k", _)` is `insert`,
  with `map` and `"k"` before the value;
- the left side is the expression the binding is bound to,
  and its range is what is written on the left;
- each `_` is a reference to the binding, and reads as the `_` it stands for.

The binding is a node the lowering makes up,
under a name no module can write ([ADR-0013][0013-pipeline-operator.md]),
and it is about the value it binds, which is written on the left.

### The source map is the record of the written form

The lowering records where each node it makes is written ([mlkc-lower]),
and that map is what keeps the spelling: for the reading of the HIR, for a host, and for a diagnostic:

- a node read from a piece of the text keeps the range of that piece;
- a node the lowering made up _for_ a piece of the text reads as that piece:
  the binding of a piped value reads as the value,
  and a reference to it reads as the `_` it stands for;
- a node the lowering made up from nothing has no range
  (an expression the parse did not find, the wildcard of a sequence);
- a node a spelling becomes is about the spelling as a whole:
  the `let` of `e1; e2` is where `e1; e2` is,
  as the expression a pair of parentheses holds is where the parentheses are;
- a piece of the spelling may become no node at all, as punctuation becomes none:
  the `;` of a sequence, and the `|>` of a pipeline.

A range is not a name: several nodes may be about one piece of the text,
as the value of a pipeline and the binding that stands for it are.

The order of the written pieces is not the order of the meaning:
a pipeline writes its value before the call and passes it inside the call,
and a spelling may write one piece in several places.
A host reads which piece became which node,
and where that node stands among the parts of the meaning.

### Diagnostics keep the quality of the construct the spelling means

The bar: a mistake about a spelling is reported the way a mistake about the construct it means would be,
and no worse.
A mistake about `receiver.function(a)` is the one a reader of `function(receiver, a)` would get,
and a mistake about `x |> f(a, _)` is the one a reader of `let v = x in f(a, v)` would get.

What follows from the split:

- what is wrong with a spelling _as syntax_ is the parser's mistake, and it points at the spelling:
  a dot that is not followed by a call, a `;` with nothing after it,
  a `_` outside the arguments of a call a pipeline applies;
- what is wrong with its _meaning_ is a mistake of the stage that owns the meaning,
  and it points at the pieces of the spelling:
  the callee, the arguments, the value a pipeline passes, the sequence as a whole;
- a diagnostic never points at a node the text does not have:
  a node the lowering made up from nothing has no range,
  and a range is what a diagnostic points with.

The spelling is not simulated in words.
A message is phrased about the meaning —
a call passes the values it passes, the receiver first ([ADR-0012][0012-split-the-dot-operator.md])
or a piped value where its place is —
and what the reader reads under the label is the text they wrote.

Nothing is lost by this:
two spellings of one meaning are, by admission, spellings of the _same_ construct,
so a mistake in one is a mistake in what both of them mean.
A message that would have to say "after the dot" would be a message about syntax,
and syntax is the parser's.

### A host reads the spelling from the syntax and the meaning from the HIR

A host that marks a buffer is given both:
the syntax, which is what was written, and the HIR, which is what it means ([mlkc-wasm]).
The body source map is the correspondence between them.

- A position in the buffer is a node of the syntax,
  and the node of the HIR that reads as that range is what the piece means:
  what a definition, a type, or a signature is read from.
  A piece may be about no node — the `;` of a sequence,
  the `|>` of a pipeline —
  and then a semantic feature has nothing to answer there.
- A node of the HIR is a place in the buffer, or nothing.
  A node the lowering made up from nothing is a node the text does not have,
  and a host has nothing to mark for it,
  which is the shape the reading of the HIR already has:
  a line is about a range, or about nothing.
- A feature that needs the shape of the spelling —
  which operator stands at the caret, how many arguments are written in the parentheses —
  reads the syntax; a feature that needs what it means reads the HIR.
  Completion is the operator's business ([ADR-0012][0012-split-the-dot-operator.md]) and the scope's;
  signature help is the callee's signature and the place of the argument at the caret
  among the arguments of the call,
  and that place is read from the meaning: the argument is a node of the call,
  and where it stands there is the parameter,
  however the spelling wrote it — a receiver before the arguments, a piped value inside them.

### What a spelling costs

A spelling is added by:

1. a node of the grammar and its parsing:
   the spelling is written, so it is shaped and lossless like every other form ([ADR-0002][0002-lossless-syntax-tree.md]);
2. a rule in the lowering:
   the one place that reads the spelling and makes what it means,
   and the only code a spelling adds to the pipeline.
   The rule may make up a binding, as a pipeline that uses its value twice does,
   and a binding it makes up is under a name no module can write;
3. what the map records:
   which pieces of the spelling become nodes with ranges of their own,
   what the node the spelling becomes is about,
   what each node the rule makes up stands for,
   and what the rule makes up from nothing;
4. the mistakes the spelling itself can have:
   they are the parser's, and they point at the spelling;
5. the mistakes what it means can have:
   they are the stages' that own the meaning, and they point at the written pieces;
6. a note for a host:
   how a position in the spelling maps to the nodes it means,
   and what, if anything, a host offers at the tokens of the spelling;
7. the tests of a lowering:
   the spelling and the written construct lower to the same HIR,
   and what each node is about is read from the map.

Points 1 and 2 are the whole of the code; the rest is what a reviewer of a spelling checks.

### Positive Consequences

- One construct per meaning:
  the resolver, the checker, and the codegen have one case each, and the case is the meaning.
  A spelling added later costs a grammar node, a lowering rule, and nothing else.
- Two spellings of one meaning lower to the same HIR,
  so the tests, the reading of the HIR, and the equality of two bodies see one thing.
- A spelling writes its pieces where it reads best:
  a pipeline puts its value before the call and passes it inside,
  and a piece may be written in several places and be one value.
- Syntax mistakes and meaning mistakes stay apart:
  the parser speaks about the spelling, and the stages speak about the meaning,
  and neither has to know how the other spells anything.
- A host keeps its mapping:
  the source map is the correspondence it already reads,
  and a node with no range is a shape it already handles.
- A spelling cannot cross a module boundary:
  an interface carries entities, signatures, and imports ([ADR-0008][0008-compiler-driver.md], [ADR-0010][0010-stable-entity-identity.md]),
  and a body is read where it is written.

### Negative Consequences

- The HIR is not the text:
  a reading of it shows a `let` where a `;` was written,
  and no spelling is recoverable from the HIR alone.
  A consumer that must tell the two spellings apart reads the syntax at the node's range.
- A node the lowering made up from nothing is a shape everything that walks a body has to have an answer for;
  the answer is "there is nothing to mark", and the lowering is the only stage that makes one.
- The lowering carries one case per spelling, by hand:
  a mistake in a rule is a mistake in what the spelling means wherever it is used.
- A message counts what the meaning counts:
  the arity of `receiver.function(a)` is the arity of `function(receiver, a)`,
  the receiver among its arguments,
  and a pipeline passes the values the call it makes passes, the value among them.
  A count is the one the written construct would get,
  and the pieces the message points at are the ones the reader wrote.
- The order of the text is not the order of the meaning,
  so a feature that walks a spelling and its meaning in step walks both:
  a pipeline's value is written before the call and passed inside it.

### What this record does not decide

- Which spellings the language should have.
  This record decides what a spelling costs once the language has it.
  The admission rule is the one above: a spelling means a construct the language writes directly,
  so a new _meaning_ is a construct of its own and not a sugar.
- The way the rules are written:
  by hand, as they are today, or from a declaration a generator turns into lowering code.
  This record decides where a spelling stops, not how its rule is spelled in the code.
- Sugar a module declares for itself:
  the language has no macros, and user-defined sugar is a language feature, not a compiler's.

## Pros and Cons of the Options

### Every stage knows every spelling

The HIR keeps a variant per spelling — `Seq` for a sequence, a node for a dot call —
and each pass handles each of them.

- Good, because each pass can phrase its diagnostics about the exact spelling.
- Good, because a host reads the spelling from the HIR without asking the syntax.
- Bad, because a construct is implemented once per spelling per pass:
  with N spellings and M stages, N × M places to keep in agreement,
  and a rule fixed in one of them is a bug in the others.
- Bad, because the HIR stops being the meaning:
  every consumer that reads a body meets as many shapes as the language has spellings,
  and two bodies that mean the same are not the same value.
- Bad, because the cost of a spelling grows with the size of the pipeline.

### The parser rewrites the spelling

The tree holds the meaning: `e1; e2` is parsed as a `let`, and the `;` is kept as trivia.

- Good, because everything after the parser sees one construct, lowering included.
- Bad, because the tree stops being a function of the text,
  which is what the identity of a parse is ([ADR-0002][0002-lossless-syntax-tree.md], [ADR-0007][0007-vfs-file-state.md], [ADR-0008][0008-compiler-driver.md]).
- Bad, because the nodes the parser makes for the meaning have no text of their own:
  their ranges point at nothing or at text that is not theirs —
  the `let` keyword of a sequence —
  and the reader loses what they wrote before any stage could report on it.
- Bad, because the same family was rejected for the prelude ([ADR-0011][0011-module-prelude.md]).

### A rewrite pass of its own

Lowering reads the spelling into a surface HIR, and a pass after it rewrites the surface into a core.

- Good, because the lowering stays a faithful reading of the syntax, one node per node of it.
- Good, because the rules of the spellings are a pass of their own,
  pure and testable like any other ([ADR-0009][0009-pass-contract.md]).
- Bad, because there are two HIRs, and every consumer has to know which one it reads.
- Bad, because the rewrite changes the arenas:
  the ids of the core are not the ids of the surface,
  so every map, every diagnostic, and every id a host holds has to be translated.
- Bad, because the surface HIR exists only to be discarded.

### Declare the rules once, and generate the code from them

A table says what each spelling lowers to —
which node, which pieces in which order, what is made up —
and the lowering, and the notes a host needs, follow from it.

- Good, because a spelling is declared in one place, and nothing is written twice.
- Bad, because it is a language and a generator of its own,
  and the spellings of this record are one line of lowering each:
  the machinery is far larger than what it serves.
- Bad, because the declaration has to say what the hand-written rule says anyway,
  which is the same work in a notation a reader has to learn first.
- Bad, because the language has no macros,
  and building the machinery for sugar a module declares is a language decision, not a compiler one.

### Lowering is the boundary

Chosen. Described above.

- Good, because a spelling costs one rule in one place,
  and every stage after lowering reads one construct per meaning.
- Good, because the written form is kept where a host already looks:
  the source map, the correspondence between the HIR and the text.
- Good, because diagnostics are about written pieces:
  the map says what each node is about, and a node the text does not have is not a place.
- Bad, because the HIR and the map do not say which spelling was written,
  so a consumer that must tell `e1; e2` from `let _ = e1 in e2` reads the syntax at the node's range.
- Bad, because the lowering grows one case per spelling,
  and the rule that keeps it from growing a construct as well is a rule a reviewer has to hold.

## Links

- The spellings decided so far: [0012-split-the-dot-operator.md], [0013-pipeline-operator.md]
- Lossless syntax and the shape of a node: [0002-lossless-syntax-tree.md]
- Versions and the identity of a parse: [0007-vfs-file-state.md]
- The driver, and what a host is handed: [0008-compiler-driver.md]
- What a pass may read, and how it is tested: [0009-pass-contract.md]
- Module rules and what an interface carries: [0004-module-system.md], [0010-stable-entity-identity.md]
- The same argument, against injecting syntax: [0011-module-prelude.md]
- Implementation: [mlkc-lower], [mlkc-hir-def], [mlkc-wasm]

[0002-lossless-syntax-tree.md]: 0002-lossless-syntax-tree.md
[0004-module-system.md]: 0004-module-system.md
[0007-vfs-file-state.md]: 0007-vfs-file-state.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0010-stable-entity-identity.md]: 0010-stable-entity-identity.md
[0011-module-prelude.md]: 0011-module-prelude.md
[0012-split-the-dot-operator.md]: 0012-split-the-dot-operator.md
[0013-pipeline-operator.md]: 0013-pipeline-operator.md
[mlkc-lower]: ../../crates/mlkc-lower
[mlkc-hir-def]: ../../crates/mlkc-hir-def
[mlkc-wasm]: ../../crates/mlkc-wasm
