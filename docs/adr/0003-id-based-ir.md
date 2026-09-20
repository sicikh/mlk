# Represent IRs as ID-based trees in per-owner arenas

- Status: accepted
- Date: 2026-09-20

## Context and Problem Statement

The CST is lossless but syntactic:
it preserves every token and trivia,
and nodes relate to each other positionally, not semantically.

The HIR must express semantic relations:
a path expression must link directly
to the definition it refers to.

A straightforward tree of boxed or reference-counted nodes
would make such links possible,
but at a cost:

- nodes scattered across the heap are cache-hostile;
- lifetimes of interlinked nodes are painful in Rust;
- references cannot outlive a rebuilt subtree,
  which fights granular incrementality.

We need a representation
that stores nodes densely,
refers to them cheaply,
and makes semantic cross-references plain data.

## Decision Drivers

- Semantic linking: nodes must refer to other nodes.
- Data-oriented design: memory is owned by central structures,
  grouped by kind, not scattered across small objects.
- Performance: dense memory layout, small references,
  cheap bulk allocation and deallocation.
- Incrementality: the invalidation unit of a tree
  must match the natural unit of an edit.
- Ergonomics: no lifetime tangle in Rust;
  references are copyable values.
- Precedent: rust-analyzer's `hir-def`
  uses exactly this design in production.

## Considered Options

- **Boxed or reference-counted tree** — the classic ADT:
  every node owns its children through heap pointers.
- **Arenas with newtyped indices (ID-based trees)** — flattened nodes,
  references are typed IDs tied to the arena's content type.

## Decision Outcome

Chosen option: "Arenas with newtyped indices (ID-based trees)",
because it combines the performance of flattened storage
with the safety of typed references,
and it makes semantic linking across trees natural.

The design lives in [mlkc-hir-def].
The principles follow.

### A tree is a set of arenas, each with one owner

Each kind of node is stored in its own arena:
`Arena<Expr>`, `Arena<Pat>`, `Arena<PathData>`, `Arena<TypeRef>`,
and one arena-map per kind of definition.

A node is an entry in an arena,
and a reference to a node is an ID into that arena.

The arenas are grouped by owner:

- an `ItemTree` per module
  holds the module's definitions, paths, and type references;
- a `Body` per function
  holds the function's expressions and patterns,
  together with the definitions of local items.

Expressions of nested local functions
are stored in the enclosing body's arena:
they are rebuilt together with it.

```rust
pub struct Body {
    /// Expressions of this body.
    exprs: Arena<Expr>,
    /// Patterns of this body.
    pats: Arena<Pat>,
    /// The root expression of this body.
    root: ExprId,
}
```

### IDs are newtyped indices

An ID is a newtype over an arena index:
`ExprId = Idx<Expr>`, `PatId = Idx<Pat>`, and so on.

The newtype is a proof:
an `ExprId` is evidence that an `Expr` exists,
and it can never be confused with a `PatId` —
mixing the two is a compile error, not a runtime error.

IDs are plain data:
`Copy`, `Hash`, `Eq`, serializable, orderable.
An ID occupies 32 bits,
and is option-optimized:
`Option<ExprId>` is no larger than `ExprId`.

The arena itself comes from [mlkc-la-arena],
a vendored copy of rust-analyzer's `la_arena`.

### Cross-tree references are composite keys

An ID is unique only within its arena.
A reference that must be unique across the whole project
combines the ID with the identity of the arena's owner:
`ModuleItemKey { module_id, id }` for module-level items,
`LocalItemKey { module_id, owner_id, id }` for items inside a body.

A composite key is a product of proofs:
the key proves _which_ arena holds the node,
the ID proves _where_ in that arena it is.

### Arena placement encodes invalidation granularity

Changes invalidate the arena they touch:

- editing an expression rebuilds the arena of its `Body`,
  and only that arena;
- editing a module-level item rebuilds that module's `ItemTree`.

The expression arena therefore lives in `Body`, not in `ItemTree`.
If it lived in the module,
every expression edit would rebuild the module tree
and force recompilation of the whole module.
Because it lives in the body,
an expression edit invalidates exactly one function.

Rule of thumb:
an arena lives with the smallest owner
whose invalidation unit contains it.
The arena hierarchy mirrors the hierarchy of compile units.

### Positive Consequences

- Semantic cross-references are cheap, stable, serializable data.
- No lifetime gymnastics:
  IDs are `Copy`,
  and each arena has one lifetime.
- Flattening gives cache-friendly traversal
  and small references;
  children are allocated before parents,
  so linear scans visit them in dependency order.
- The compiler rejects mixed-up IDs at compile time.
- Invalidation granularity falls out of arena placement,
  which keeps incremental recompilation fine-grained.
- Nodes are addressable by ID,
  which simplifies debugging and serialization.

### Negative Consequences

- Every access goes through an indexing step
  (`arena[id]`) instead of a direct field;
  pattern matching requires fetching the node first.
- IDs carry no generation counter:
  an ID from a previous build of an arena
  may silently refer to a different node
  after the arena is rebuilt.
  Code must not keep IDs across rebuilds.
- Allocation and access require boilerplate,
  partly mitigated by the macros.
- An ID is only meaningful together with its arena:
  accessors must be designed per owner.

## Links

- Handles are the better pointers: <https://floooh.github.io/2018/06/17/handles-vs-pointers.html>
- Newtyped indices are proofs: <https://eikopf.bearblog.dev/newtyped-indices-are-proofs/>
- Flattening ASTs (and other compiler data structures): <https://www.cs.cornell.edu/~asampson/blog/flattening.html>
- Implementation: [mlkc-hir-def]
- Arena library: [mlkc-la-arena]

[mlkc-hir-def]: ../../crates/mlkc-hir-def
[mlkc-la-arena]: ../../crates/mlkc-la-arena
