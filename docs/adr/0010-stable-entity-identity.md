# Give HIR entities stable identities

- Status: accepted
- Date: 2026-09-26
- Supersedes: [ADR-0003](0003-id-based-ir.md)

## Context and Problem Statement

[ADR-0003][0003-id-based-ir.md] fixed the representation of the HIR:
nodes flattened into arenas, newtyped indices that refer to them,
and one arena group per owner.
That representation stands, and this record keeps it;
what it changes is identity.

An index is a position in an arena of one revision of one owner,
and the composite key of [ADR-0003][0003-id-based-ir.md] --
`ModuleItemKey { module_id, id }` -- is a position as well.
The compiler cannot use a position as the identity of a cached unit:

- the driver addresses a memo entry by the identity of the unit it holds,
  and decides validity by comparing identities, never fingerprints
  ([ADR-0008][0008-compiler-driver.md]);
- a host asks for one function, one body, one signature,
  and the answer has to survive edits to everything else in the module,
  or nothing is reused between two keystrokes.

Positions fail that in two different ways.

**A position is not a name.**
Insert `f` before `g`, and the key of the unit "the body of `g`" changes:
the entry that held `g` is no longer found under a key that means `g`,
and the key that used to mean `g` now denotes `f` or the item after it.
A cache keyed by a position is not merely ineffective, it is wrong:
the key does not prove what it names.

**A position inside a value breaks the comparison of values.**
[ADR-0008][0008-compiler-driver.md] decides validity by comparing the values a unit was built from,
and recomputes a unit whose inputs changed.
A positional id stored inside such a value --
a resolved path in a signature, an entry of a module scope, a reference to a type --
shifts when an unrelated item is inserted in the module it points into.
Two semantically identical values then compare unequal,
the driver cannot back-date,
and invalidation travels from one inserted item to every dependent of the module.

The question:
what has to be true of the identity of an entity,
of the observable data of an entity,
and of a body,
for a cache entry to survive an edit it does not observe?

## Decision Drivers

- Incrementality: the identity of a cached value changes exactly when the meaning of that value changes.
- Granularity: the units are entities, not modules;
  an edit to one function re-runs that function and its readers, and nothing else.
- Exactness: validity is decided by identity or by exact comparison of values,
  never by a fingerprint ([ADR-0008][0008-compiler-driver.md]).
- Parallelism: values are immutable, `Send + Sync`, and shared by `Arc`;
  a pass reads nothing but its input ([ADR-0009][0009-pass-contract.md]),
  so the entities of one module are independent units of work.
- The module independence the language already bought ([ADR-0004][0004-module-system.md])
  must not be given back by naming the internals of another module.
- Determinism: no address, no allocation order, and no hash order may reach the output.
- Future reuse: an identity that can be written down and compared in another process.
- Ergonomics: handles are `Copy`, values carry no lifetimes, and nothing has interior mutability.

## Considered Options

- **Positions with generations** --
  keep arena indices and add a generation counter bumped whenever an owner is rebuilt,
  so that a stale index is detected instead of silently pointing at another node.
- **Stable structural identities** --
  an entity is named by its shape:
  its kind, its name, and its place among the entities that share both.
- **Content-addressed identities** --
  an entity is named by a fingerprint of its source text, a value by a fingerprint of its content.
- **Identities assigned by a diff** --
  when a new item tree arrives, the driver aligns it with the old one and reuses the old identities.

## Decision Outcome

Chosen option: "Stable structural identities",
because a name is a value and not a summary:
it cannot collide, it can be written down, and comparing it is exact.
It is also a function of the current text alone,
so an incremental driver and a fresh driver agree on what an entity is --
which is what the conformance test of [ADR-0008][0008-compiler-driver.md] compares.
Positions stay where they are a proof and a saving: inside the value that owns the arena.

This is the stable identity that the deferred item of [ADR-0008][0008-compiler-driver.md]
("Narrower units and entries: slots per item and per body,
and inputs that name entities") waits for.

### A position and a name are two kinds of handle

An **index** (`Idx<T>` of [mlkc-la-arena]) is a proof that a node exists,
and the proof holds only inside the value that owns the arena.
Outside it the same index proves nothing:
it may be stale, and it may denote a different node.

A **name** (the `Loc` family below) denotes an entity of a module.
Existence is not part of it: a name may denote nothing,
and every use of a name is a lookup that may fail.

Three rules follow.

- A positional id never leaves the value that owns the arena it points into.
  Inside a body it is the natural handle: `ExprId`, `PatId`,
  and the ids of the entities declared in that body.
  In a key, in an interface, or in a value that outlives a revision it is a bug.
- A name is the only identity of a unit for anything that crosses a revision:
  memo slots, entries of an interface, entries of a scope, whatever is written to disk.
- A name is resolved against a revision through the structure that defines it:
  `ItemTree` maps a name to a positional id, and the map is a function of that tree's own contents.

### An entity is named by its shape

```rust
/// One variant per kind of module-level declaration.
/// Part of an entity's identity: variants are added, never renamed or removed.
pub enum ItemKind {
    Function,
    Class,
    Value,
    Const,
    Impl,
    Use,
}

/// What every name shares, whatever the kind of the entity it names.
pub struct ItemLocData {
    /// `None` for an entity with no name of its own, such as an `impl`.
    pub name: Option<Name>,
    /// Which entity of this kind and name this is, counted from the start of the owner.
    /// Zero, unless the name is duplicated --
    /// an error that still has to be nameable while the module does not compile.
    pub disambiguator: u32,
}

/// The name of a function inside its owner.
/// One of these per kind, generated from the same list as `ItemKind`:
/// `ClassLoc`, `ValueLoc`, `ConstLoc`, `ImplLoc`, `UseLoc`.
pub struct FunctionLoc(ItemLocData);

/// The name of an entity of any kind:
/// what a scope, an interface, or a memo table keys on.
pub enum ItemLoc {
    Function(FunctionLoc),
    // one variant per kind
}

/// The name of an entity that owns a body: a subset the language defines.
pub enum BodyLoc {
    Function(FunctionLoc),
    Const(ConstLoc),
}

/// What all of the above share, so that most code never matches on a kind.
pub trait ItemLocLike {
    fn data(&self) -> &ItemLocData;
    fn kind(&self) -> ItemKind;
}

/// The name of an entity in the project.
pub struct EntityLoc<I = ItemLoc> {
    pub module: ModuleId,
    pub item: I,
}

/// The name of an entity that owns a body, in the project.
pub type BodyEntityLoc = EntityLoc<BodyLoc>;
```

What the name is derived from, and what follows:

- **the shape of the item tree, not the syntax and not the arena order**:
  the name of an entity is the same in every revision in which the module declares
  an entity of that kind and name, however the module was edited around it;
- **inserting, deleting, moving, and reordering other entities changes nothing**,
  which is the property that makes a cache entry survive an unrelated edit;
- **renaming an entity makes it a new entity**.
  That is deliberate: the readers of the old name have to be re-checked anyway,
  and a name that outlives a rename would need an identity that is a function of history
  rather than of text, which the last option in this record is rejected for;
- **a duplicated name is disambiguated by a counter**, counted among the entities
  of the same kind and name in source order.
  Duplicates are an error, so the counter only has to be nameable, not meaningful;
- **an anonymous entity is named by its place among the anonymous ones of its kind**.
  Inserting an `impl` before another therefore makes the second one a new entity.
  Deriving its name from the head of the `impl` instead is possible and deferred;
- **a local entity -- a function or a constant declared inside a body -- gets no name at all**.
  It is not observable outside its owner, and the identity of the enclosing body
  already pins every positional id inside it.
  An entity needs a name exactly when it is observable outside the value that holds it.

Two more names complete the rule, one level up and one level down:

- a **module** is named by its file:
  the id the VFS interns by path is never reused,
  and the path itself is the canonical form of the name ([ADR-0007][0007-vfs-file-state.md]);
- a **type variable** of an entity is named by the name of its owner and its index in the declaration.
  It is the same rule again: a name to cross the boundary of an owner, a position inside it.

### A name is typed where the kind is known, and erased where it is not

The family is not decoration: it turns the sets the language already defines into constraints.

- **One typed name per kind** -- `FunctionLoc`, `ClassLoc`, `ValueLoc`, `ConstLoc`, `ImplLoc`, `UseLoc` --
  generated from the same list as `ItemKind`.
- **One erased name** (`ItemLoc`) for a position that has to accept any kind:
  a resolved path, an entry of a scope, an interface, the address of a memo slot.
  Such a position cannot be typed, and pays a match where the kind matters.
- **One typed name per subset of kinds the language defines**:
  `BodyLoc` for the entities that own a body,
  and the same shape where a scope has namespaces.
  This is where the typing earns its keep:
  the guarantee worth having is "a body slot cannot name a class",
  more than "a class cannot be used where a function is expected".
- **One project-wide name**: `EntityLoc<I>` -- a module and a name inside it --
  with `BodyEntityLoc = EntityLoc<BodyLoc>` for the units that are bodies.
  The default parameter keeps the common case spelled `EntityLoc`.

Four rules keep the forms interchangeable and exact.

- **Widening is `From`; narrowing is checked.**
  A typed name widens into the erased one for free.
  The other direction is a `TryFrom`, except where the arena is the proof:
  `ItemTree::function_loc(ModuleFunctionId) -> FunctionLoc` is total in the revision that holds the id.
- **`Eq`, `Hash` and `Ord` agree between the forms**, and all three start with the kind,
  so a map keyed by the erased name is looked up with a value widened from a typed one.
  The order is `(kind, text of the name, disambiguator)` inside a module,
  and a module is the outermost key of `EntityLoc`.
  That order is what gives a scope, an interface,
  or a list of use sites in a diagnostic the same shape in every process.
- **A name is never built by hand.**
  It comes from the item tree, which owns the disambiguation counter,
  or from a checked narrowing of a name that came from there.
  The fields are private for this reason, as the fields of the positional ids were.
- **An API offers both forms**: one erased entry point for code that means "an entity",
  and one typed accessor for code that means "this kind of entity".
  The typed accessor is where a wrong kind stops being a `None` and becomes a compile error.
  A key type can require a name this way too:
  a bound that only the names implement makes a positional id in a driver key not expressible.

### The item tree is a module's surface, addressable entity by entity

The item tree of a module holds what the module's text alone determines:

- the entities of the module and their observable data;
- the import table, as the unresolved paths of its `use` items;
- the path the module declares itself as, if its preamble writes one:
  the name the project knows the module by,
  of which the file the module is read from is the canonical form;
- the bijection between names and arena ids, so that either direction of lookup is one step;
- where each entity is in the module's syntax (see below).

It holds no body, and it holds nothing that an edit inside a body can shift:
this is the constraint of [ADR-0008][0008-compiler-driver.md]
("an item tree plus an `AstIdMap`, never the ranges"),
and here it becomes a property of identity rather than a storage preference.
Text ranges, node counts, and anything else measured over the whole file are out:
a comment inside one function must not change what another function is called.

```rust
impl ItemTree {
    /// The entities of the module, in source order.
    pub fn entities(&self) -> impl Iterator<Item = (ItemLoc, ModuleDefId)> + '_;

    /// The path the module declares itself as, if it has a preamble.
    pub fn path(&self) -> Option<PlainPathId>;

    /// The erased entry points: for code that means "an entity".
    pub fn entity_data(&self, item: ItemLoc) -> Option<&EntityData>;
    pub fn id_of(&self, item: ItemLoc) -> Option<ModuleDefId>;
    pub fn loc_of(&self, id: ModuleDefId) -> ItemLoc;

    /// The typed accessors: for code that means "this kind of entity",
    /// and wants the compiler to agree. One pair per kind, generated.
    pub fn function_data(&self, item: FunctionLoc) -> Option<&FunctionData>;
    pub fn function_id(&self, item: FunctionLoc) -> Option<ModuleFunctionId>;
    pub fn function_loc(&self, id: ModuleFunctionId) -> FunctionLoc;

    /// The accessor of a subset the language defines:
    /// only the entities that own a body can be looked up here.
    pub fn body_id(&self, item: BodyLoc) -> Option<ModuleDefWithBodyId>;
    pub fn body_loc(&self, id: ModuleDefWithBodyId) -> BodyLoc;
}
```

**The observable data of an entity is a value of its own.**
`EntityData` -- a signature, the shape of a class, the class and the type of an `impl` --
owns what it names:

- a local target is an `EntityLoc` of the same module;
- a foreign target is what the module wrote: a path in its import table, unresolved;
- no field of it is an index into an arena shared with other entities.

Three consequences make this the unit of incrementality:

- it can be compared on its own, so a reader of a signature keys on that signature
  rather than on the whole interface of the module it came from;
- it can be retained and serialized on its own, so the entry of an interface is self-contained;
- it is small, so a comparison of a handful of signatures is cheap enough
  to be the validity check of a dependent unit.

The alternative -- one arena of paths and type references per item tree,
with entity data as a slice of it -- is what the drafts did,
and it is rejected because a slice cannot be compared, retained, or sent anywhere alone:
its meaning is a function of the tree it is cut from, which is exactly the coupling
this record removes.
The price is that an identical type reference written in two signatures is stored twice.
Signatures are small, and the price is paid once per module revision.

### A position is kept where the owner's value is the unit of invalidation

[ADR-0003][0003-id-based-ir.md]'s rule stands unchanged:
an arena lives with the smallest owner whose invalidation unit contains it.
Restated for the units this record names:

- a **body** owns the expressions, the patterns, the paths, the type references,
  and the entities local to it.
  An edit inside a body rebuilds that body's arenas and no others;
  the local entities inside it are addressed by positional ids,
  which is sound because their owner is a single value that is never sliced;
- an **item tree** owns the surface of one module: entities, their data, the import table, the index.
  An edit to the surface changes the item tree;
  an edit to a body leaves it equal, so it is back-dated
  and no dependent of the module is invalidated;
- a **body has no identity of its own**: it is the body of an entity,
  and the entity's name plus the syntax it was lowered from identify it.

This is also why a body is not part of an interface and not part of an item tree:
a body edit cannot change what a dependent reads,
which is [ADR-0004][0004-module-system.md]'s mandatory public signatures, seen from the HIR side.

### Resolution stays inside the module, and the link across modules is derived

[ADR-0003][0003-id-based-ir.md] asked a path to link directly to the definition it refers to.
This record refines that:
a path links to its **local anchor**, and the link beyond the module boundary is derived.

- Local name resolution ([ADR-0004][0004-module-system.md]) resolves every use of a name
  to an entity of this module, to an entry of this module's import table,
  to a binding of the enclosing body, or to a type variable.
  All four are addressable from within the module, and the first two are names by the rules above.
- Nothing in the HIR points at an entity of another module.
  A foreign name is stored as the path the module wrote,
  and it becomes an entity only when a scope resolves it.

Why this matters for incrementality and parallelism:

- a body's lowering reads the module's own text and its own item tree and nothing else,
  so lowering parallelizes over entities without consulting anything shared,
  and a body's inputs contain nothing that another module can change;
- a foreign positional id in the HIR would be an id of an owner the reader does not retain,
  or worse, it would force the reader to retain that owner to keep the id meaningful
  ([ADR-0008][0008-compiler-driver.md]'s "an id that points into another module
  therefore lives only inside a value whose key holds the interface that gives it meaning");
- resolution across modules is still a value, but a derived one:
  the scope of a module holds names mapped to `EntityLoc`s of the projects it imports,
  and the unit that holds it is keyed by the interfaces it was resolved against.
  That is the whole-project index of [ADR-0008][0008-compiler-driver.md],
  and it is a consumer of interfaces rather than a part of the HIR;
- the same rule applies to a **project**:
  a dependency is named by its path, or by the name it is declared under,
  not by its index in the project graph.
  Otherwise the entry of a scope that points at a virtual module of a dependency
  would move when an unrelated dependency is added.

### The name of an entity is recoverable from the module's syntax

A per-entity pass needs the syntax of its entity -- the signature and the body of a function --
and the driver holds a parsed file and a name.
`ItemTree` answers with a **syntax locator**:

```rust
impl ItemTree {
    /// Where in the module's syntax the entity is,
    /// as a path of child slots from the module root.
    pub fn syntax_loc(&self, item: ItemLoc) -> Option<ItemSyntaxLoc>;
}
```

- The locator is a path of slots (`items[3]`, then the body of that declaration),
  not a text range and not a count of nodes of the file.
  It is stable under every edit inside a body, which is what keeps the item tree --
  and with it the identity of every entity -- unaffected by such an edit.
- It is produced by the same shallow traversal that produces the entities,
  so there is exactly one numbering of the items, and one place to keep it right.
- It is stored inside the item tree, so its own instability under an inserted item is harmless:
  the tree it belongs to changes in exactly that case.

The alternative rust-analyzer uses -- a separate `AstIdMap`, with an id per item-shaped node --
is rejected for now, because MLK needs one mapping (entity to item syntax)
and already walks the entity-shaped nodes to build the item tree;
a second structure would be a second numbering to keep in step by hand.
It remains the way out if many more node kinds ever need ids.

### Names are interned, equality is exact, hashing and ordering are canonical

Names are interned ([mlkc-intern]), and interning is load-bearing rather than an optimization:

- it makes `Eq` on two entities, two item trees, or two signatures a handful of pointer comparisons,
  which is what lets the driver compare values at all
  ([ADR-0008][0008-compiler-driver.md] decides validity by comparing values);
- it is exact as long as the interner keeps one live allocation per text,
  which it does by construction, and as long as a compared value stays alive,
  which the driver guarantees by holding the value its key names.

Three rules keep the scheme honest.

- **`Eq` on interned names is pointer equality**,
  and it is exact under the two conditions above.
  A textual fallback was considered and rejected:
  it would make the cost of `Eq` depend on the length of a name,
  and it would hide a violation of the retention rule instead of surfacing it.
- **`Hash` and `Ord` on names are by text**, never by address.
  Map layout is then independent of allocation addresses,
  so nothing derived from an address can leak into an output through an iteration order
  ([ADR-0009][0009-pass-contract.md]).
  Ordering that reaches an output is still explicit or by text, never by hash.
- **A canonical fingerprint is a separate, hand-written function next to a value**,
  hashing names by text, for tracing, for `--stats`, and for the on-disk cache
  ([ADR-0008][0008-compiler-driver.md]).
  It never takes part in a validity decision.

### What the driver is given

This record changes no driver decision;
it supplies what [ADR-0008][0008-compiler-driver.md] and [ADR-0009][0009-pass-contract.md]
were missing. In their terms:

| Unit (addressed by a name) | Key                                                       | Value            |
| -------------------------- | --------------------------------------------------------- | ---------------- |
| `ItemTree(module)`         | the version of the module's text                          | `Arc<ItemTree>`  |
| `Entity(EntityLoc)`        | the name, the version of the module's text                | `EntityData`     |
| `Body(BodyEntityLoc)`      | the name, the syntax (the retained parse and the locator) | `Arc<Body>`      |
| `Check(BodyEntityLoc)`     | the name, `Arc<Body>`, the `EntityData` it read           | the checked body |

Three rules of the table are new, and all three follow from this record.

- A unit is addressed by a name and keyed by what it read.
  A positional id appears in neither.
- A unit that is a body is addressed by a **body name**:
  a slot for a body cannot name a class, and the compiler says so.
- An entity's data enters the key of a dependent **by value**:
  it is small, it is exact, and a value needs no retention to stay meaningful.
  The `Arc`-identity variant of [ADR-0008][0008-compiler-driver.md] remains for the big values --
  the parse, the item tree, the body -- where comparing by value would cost a copy of the whole.

What the model buys, in one example:

```mlk
// before
fun f(): Int = 1
fun g(): Int = 2

// after
fun f(): Int = 1
fun h(): Int = 3
fun g(): Int = 2
```

With positional keys, the key that used to name `g` now names `h`,
and everything derived from `g` is either stale or gone.
With names, `g` is the same entity,
`f` and `g` keep their data and their bodies,
the readers of `f` and `g` see the same values they saw,
and the only new work is the entity `h` and whatever asks for it.

### Positive Consequences

- An inserted item rebuilds the module's item tree,
  but the data of every existing entity in it is unchanged,
  so no reader of an existing entity is invalidated.
- A signature edit re-runs the readers of that signature, not the dependents of the module.
- A body edit re-lowers the module's bodies and invalidates no reader of any signature,
  because a body is not part of what a reader reads and an equal body back-dates.
- Entities are independent units of work,
  so parallelism inside a module matches parallelism across modules;
  a pass per entity reads an immutable `Arc` and needs no global state.
- An interface is self-contained and serializable:
  it holds names and entity data, so a dependent pins the data it read
  rather than the whole item tree of another module.
- Positions stay cheap -- 32 bits, dense, no validity check --
  where the owning value is the unit of invalidation.
- The identity of a unit is a value:
  it can be logged, compared in a test, and written to disk.
- A position that carries a kind is checked by the compiler:
  a body cannot be addressed by the name of a class,
  and a name of any kind is still one value that can be stored, compared, and printed.
- Names have a canonical order,
  so a scope, an interface, or the use sites in a diagnostic read the same way in every process.

### Negative Consequences

- There are two kinds of handle and a rule for choosing between them,
  and only review enforces the rule.
  A bound on the driver's key types would make "a position never enters a key" a compile error;
  this record supplies the vocabulary for it, and does not require the driver to adopt it.
- The loc family is a family: one type per kind, the erased sum, the subsets, and a trait
  to keep helpers generic.
  The generator keeps them in step; a reader has one more vocabulary to learn.
- Two forms of one name exist,
  and their `Eq`, `Hash` and `Ord` have to be kept in agreement by the generator rather than by review.
- A name costs a lookup through the item tree's index,
  where an index into an arena was free.
  The lookup is paid once per unit per revision, not once per node.
- Two identical type references in two signatures are stored twice,
  because entity data owns what it names.
- Renaming an entity is a delete and an insert for the cache:
  the work of the old name is dropped, and the readers of the old name re-run.
- An anonymous entity moves when an anonymous entity is inserted before it.
- The model leans on the language:
  an entity's data has to be a function of its own module's text
  ([ADR-0004][0004-module-system.md]).
  A feature that lets a body or another module change that data
  would show up here as a coarser cutoff, not as a wrong answer.
- The driver has to drop the slots whose names are gone from the current item tree,
  which is bookkeeping the positional draft did not need.

## Pros and Cons of the Options

### Stable structural identities

Chosen. An entity is named by its kind, its name, and its place among its namesakes.

- Good, because a name is exact: it is a value, not a summary, so it cannot collide.
- Good, because it is a function of the current text,
  so two drivers that hold the same text agree on the identities in it,
  which is what makes the conformance test of [ADR-0008][0008-compiler-driver.md] possible.
- Good, because an unrelated edit leaves every existing name, and hence every key, untouched.
- Good, because it is serializable and readable, without a table to translate it.
- Bad, because it costs a family of types beside the index,
  and the discipline of choosing between the typed name and the erased one.
- Bad, because a rename is a new entity.
- Bad, because an anonymous entity has no shape to be named by.

### Positions with generations

Arena indices keep their meaning checked by a generation counter,
as handles are checked against a table of live objects.

- Good, because a stale index fails loudly instead of denoting another node,
  which is a real hazard of the positional drafts
  ([ADR-0003][0003-id-based-ir.md]'s "code must not keep IDs across rebuilds").
- Good, because nothing about identity is invented:
  the driver keeps keying on positions, and the counter only makes the failure visible.
- Bad, because it does not make anything stable:
  inserting an item into a module still changes what every later index means,
  so every entity after it is a new unit and nothing is reused.
  The generation only turns a silent wrong answer into a panicked one.
- Bad, because an id grows a counter, and every retained value pins the generation it was built in.
- Kept as a possible refinement _inside_ a body,
  where ids are positional anyway and a stale one would be a bug of the owner rather than a cache miss.

### Content-addressed identities

An entity, or a unit, is named by a fingerprint of its own content.

- Good, because equal content gives equal identity, whatever the edit history was.
- Good, because a fingerprint is the shape a cross-run cache wants on disk.
- Bad, because a summary can be wrong in the direction that matters:
  if it misses a part of the value a consumer observes,
  the dependent is not invalidated and is served a stale answer --
  the one failure a compiler cache must not have ([ADR-0008][0008-compiler-driver.md]).
- Bad, because a fingerprint of the source text changes on a comment or a whitespace edit,
  and a fingerprint of the extracted data cannot be computed before the data is,
  so it saves comparison time and never computation.

### Identities assigned by a diff

The driver keeps the previous item tree, aligns it with the new one,
and gives a new entity the identity of the old entity it matches.

- Good, because a rename would keep the identity of the renamed entity,
  and its body and its readers would survive a rename.
- Good, because it needs no rule about what the name of an entity is.
- Bad, because the identity of an entity becomes a function of the history of the driver,
  not of the text it holds:
  two drivers that reached the same text by different edits would disagree about what its entities are,
  and the conformance test of [ADR-0008][0008-compiler-driver.md]
  -- which compares an incremental driver against a fresh one -- would have nothing to compare.
- Bad, because the alignment is a heuristic, and a wrong alignment is a wrong answer
  of exactly the kind the cache must never give.
- Bad, because it makes the driver hold the previous revision to interpret the current one.

## Links

- Superseded record: [0003-id-based-ir.md]
- Refined by: [0011-module-prelude.md] (an entity the module did not write has no syntax)
- The driver this identity serves: [0008-compiler-driver.md]
- The contract of a pass: [0009-pass-contract.md]
- Incrementality and the language rules it relies on: [0004-module-system.md]
- Pipeline and units: [0005-compiler-pipeline.md]
- Implementation: [mlkc-hir-def], [mlkc-la-arena], [mlkc-intern]
- Handles are the better pointers: <https://floooh.github.io/2018/06/17/handles-vs-pointers.html>
- Newtyped indices are proofs: <https://eikopf.bearblog.dev/newtyped-indices-are-proofs/>
- Flattening ASTs (and other compiler data structures): <https://www.cs.cornell.edu/~asampson/blog/flattening.html>
- rust-analyzer's item tree, the positional precedent this record departs from:
  <https://github.com/rust-lang/rust-analyzer/blob/master/crates/hir-def/src/item_tree.rs>

[0003-id-based-ir.md]: 0003-id-based-ir.md
[0004-module-system.md]: 0004-module-system.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0007-vfs-file-state.md]: 0007-vfs-file-state.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0011-module-prelude.md]: 0011-module-prelude.md
[mlkc-hir-def]: ../../crates/mlkc-hir-def
[mlkc-intern]: ../../crates/mlkc-intern
[mlkc-la-arena]: ../../crates/mlkc-la-arena
