# Resolve types into self-contained values, and check them with a temporary Hindley–Milner checker

- Status: accepted
- Date: 2026-10-01

## Context and Problem Statement

The pipeline of [ADR-0005][0005-compiler-pipeline.md] has no stage past name resolution today.
Type checking is the next one,
and nothing after it exists:
MIR, layout, codegen,
and the IDE answers that have to say what an expression is
all read the types that checking produces.
The module system adds a second consumer:
a module that calls a function of another module needs to know what the call takes and gives,
and [ADR-0016][0016-inter-module-resolution.md]'s interface deliberately holds names, not types,
so a type has no representation with which to cross a module boundary at all.

At the same time, the language is not settled:
classes with parameters, `impl`s, sums and products, and their inference rules
are still to be designed,
and a checker written today will be replaced before any of them land.
The back end does not have to wait for that,
so the first checker is a temporary one.
The one thing about it that has to last
is what it leaves behind: the resolved types.

The questions this record answers:

- what is a resolved type ---
  what may it name, what may it contain, and what does equality of two of them mean;
- what does a check leave behind, and what may cross a unit boundary;
- what is the first checker, and what about it is deliberately temporary.

## Decision Drivers

- Incrementality ([ADR-0004][0004-module-system.md], [ADR-0008][0008-compiler-driver.md]):
  a dependent is keyed by what it read,
  so what a check leaves behind must be values a driver can retain, compare, and back-date.
- The unit of checking is a body:
  checking a body must not need the _result_ of checking another body,
  which is what keeps the bodies of a module independent units of work, in parallel,
  with no order between them.
- The two kinds of handle of [ADR-0010][0010-stable-entity-identity.md]:
  a positional id points into an arena of one owner and never crosses a revision;
  a name is stable, and anything that crosses a revision keys on a name.
- A value read from another module must stay meaningful
  without the arenas of that module ([ADR-0009][0009-pass-contract.md]).
- Purity and determinism of a pass ([ADR-0009][0009-pass-contract.md]):
  no global table, and nothing derived from allocation or interning order in the output.
- The language will grow:
  a feature must add variants and rules to the representation of a type,
  never replace it.
- The checker is temporary:
  nothing of unification, substitution, or levels may be visible in a stored type.
- Diagnostics carry facts ([ADR-0009][0009-pass-contract.md]),
  and a type is one of the facts:
  a diagnostic must be renderable without the state that produced it.
- Precedent:
  rust-analyzer separates the written `TypeRef` from a resolved type of `hir-ty`,
  and ML separates a type scheme from the type it generalizes.

## Considered Options

A resolved type is:

- **a self-contained value** --- a recursive enum, owned, compared structurally;
- **a positional id into a per-module type store** ---
  the flattened arenas of [ADR-0003][0003-id-based-ir.md], applied to types;
- **a handle of one global interner** --- hash-consed types, equality by pointer;
- **no resolved language at all** --- `TypeRef` stays the representation,
  and every consumer resolves it on demand.

The first checker is:

- **bidirectional Hindley–Milner, one body at a time** ---
  `infer` where the type is open, `check` against a known type, level-based generalization,
  every signature of the module written out, and every body checked on its own;
- **one check per module** ---
  a signature may be inferred from a body, at the price of checking the module as one unit;
- **Algorithm W** ---
  inference only: an annotation is unified like any other type and never drives checking;
- **nothing yet** --- the back end waits for the language to settle.

## Decision Outcome

Chosen option: "a self-contained value" for types,
and "bidirectional Hindley–Milner, one body at a time" for the temporary checker,
because together they keep every later stage,
and every later version of the checker,
from depending on how types are computed today:
the representation is a language of its own,
and the temporary checker is one of its producers.

### A resolved type is a value

The representation is a crate of its own (`mlkc-hir-ty`), above the HIR:

```rust
/// A type, resolved: no path is left unresolved, and no inference variable survives.
pub enum Ty {
    /// The type of something the checker could not type: broken syntax, a name that denotes
    /// nothing, a mistake already reported. It absorbs what it meets, so one mistake is one report.
    Error,

    /// A class, applied to its arguments: `Int`, `Map[Int, Str]`.
    Class {
        /// The class, named the way the project names it.
        class: EntityLoc<ClassLoc>,
        /// The arguments, in the order they are applied.
        args: Vec<Ty>,
    },

    /// A function: what it takes, and what it gives back.
    Fn {
        /// The parameters, in the order the function takes them.
        params: Vec<Ty>,
        /// The result.
        ret: Box<Ty>,
    },

    /// A parameter of the entity the type belongs to: the `a` of `forall a. a -> a`.
    Param(TypeVarId),
}
```

Three choices in this enum are worth stating as rules.

- **A class is a name, not a path.**
  `EntityLoc` is [ADR-0010][0010-stable-entity-identity.md]'s stable identity of an entity,
  so two modules that write a class in different ways resolve to the same value,
  and a type stays meaningful after the module that declared the class is rebuilt.
- **No type is primitive.**
  `Int` and `Unit` are classes of the standard library,
  and every type the language adds will be a class or a new variant;
  what the checker knows about a builtin is a rule of the checker, not a variant of `Ty`.
  When `impl`s arrive, the rules change and the representation does not.
- **The only variable that survives a check is a parameter.**
  A parameter is the HIR's `TypeVarId` ---
  the entity that owns it, and its index in that entity's declaration ([mlkc-hir-def]) ---
  which is exactly what a dependent records when it reads a signature.
  An inference variable, a substitution, a level, a region:
  none of them is in `Ty`, and none of them can be,
  because the type does not know that a checker exists.

A `Ty` owns everything it names,
so it clones, compares, and serializes on its own.
Equality is structural,
and its leaves are interned names compared by pointer ([ADR-0010][0010-stable-entity-identity.md]),
so comparing two types costs a walk of their shape and nothing else.
Sharing (`Arc`) may be introduced inside the tree as an optimization,
and must stay invisible to equality.

### What a check leaves behind

The types of a module are what its readers read, and what its own bodies read:

```rust
// mlkc-hir-ty
/// The types of a module's entities, as the modules that read it see them.
///
/// Resolved from the signatures the module writes and from nothing else: no body is read to
/// build one.
pub struct ModuleTypes {
    /// The type of every entity that has one, in the order the module declares them.
    types: IndexMap<EntityLoc, Ty>,
}

/// What checking one body left behind.
pub struct CheckedBody {
    /// The type of every expression of the body.
    expr_types: FxHashMap<ExprId, Ty>,
    /// The type of every pattern of the body.
    pat_types: FxHashMap<PatId, Ty>,
    /// The type of every entity declared inside the body.
    local_types: FxHashMap<LocalDefId, Ty>,
}
```

What the surface is, and what it is not:

- **The surface is what crosses a module boundary, and it is a value**, not an id.
  A reader reads a type by the `EntityLoc` its resolution already gave it,
  from the surface of the module the entity belongs to.
  A reader in another project, a body that was rewritten, an inserted declaration:
  none of them changes what `Int` of `std` is,
  because the writer's `Int` and the reader's `Int` are one name.
- **The surface is not the interface of [ADR-0016][0016-inter-module-resolution.md].**
  An interface is a function of the module's text alone --- names, visibility, re-exported paths ---
  while a surface resolves the written types through the names the module read:
  the two are read together and keyed apart,
  and a surface is a function of the module's own text and of the names it read,
  never of another module's types.
- **The surface holds every entity that has a type**, public or not:
  the module's own bodies read private signatures,
  and the modules that read the module can only name the public ones anyway.
  Splitting the surface into a public and a private projection is deferred.
- **A body is checked against the surface**,
  and what it leaves behind are types of the nodes of the body, by position inside it:
  a position is the natural handle inside a value that is rebuilt whole
  ([ADR-0010][0010-stable-entity-identity.md]).

### How a check reads the rest of the project

The check of a body resolves the paths of that body itself,
over the scope and the imports the resolution of
[ADR-0016][0016-inter-module-resolution.md] produced:

```rust
/// What checking a body reads of the rest of the project ([ADR-0009]).
pub struct CheckDeps {
    /// The projects, what each of them depends on, and the project of every module.
    pub graph: Arc<ProjectGraph>,
    /// The modules the check walks, and the interfaces of them,
    /// gathered over the surface of the module and over its bodies ([`Closure::of_check`]).
    pub closure: Closure,
    /// The type surface of the module the body belongs to, and of every module its paths name.
    pub types: BTreeMap<ModuleId, Arc<ModuleTypes>>,
    /// The classes of the language, which the literals and the operators are typed by.
    pub builtins: Builtins,
}
```

Two properties of this value matter.

- **It is a closure of what was read** ([ADR-0009][0009-pass-contract.md]):
  a path of a body may name a module that no signature of the module names,
  so the closure of a check is wider than the closure of a resolution,
  and the check walks its paths --- a module of the project, a module of another project,
  a name a module re-exports --- over the value it was handed.
  The surfaces of the modules the walk reaches are in the input too,
  its own module's among them --- a body may call any function of its module, public or not.
- **A foreign type is read by name and by value, never by id.**
  Nothing in `CheckDeps` hands out an arena of another module:
  the only foreign handles are `EntityLoc`s and the self-contained `Ty`s they map to,
  which is what lets the input survive a revision
  ([ADR-0010][0010-stable-entity-identity.md]).

One example, whole:

```mlk
// module `a`
pub fun double(value: Int): Int = ...
```

```mlk
// module `b`
fun use(): Int = a::double(1)
```

Checking the body of `use` resolves `a::double` to the `FunctionLoc` of `double` in `a`,
reads `(Int) -> Int` from the surface of `a`,
and the `Int` it compares against is the same `EntityLoc` of `std::core`
that the signature of `a` was resolved with.
No path is rewritten, no table of another module is retained,
and the two types are equal by structure alone.

### The temporary checker

The first checker is one pass per body:

```rust
// mlkc-typeck, for now
pub fn check_body(
    owner: BodyEntityLoc,
    tree: &ItemTree,
    body: &Body,
    resolution: &Resolution,
    deps: &CheckDeps,
) -> (CheckedBody, Vec<TypeDiag>);
```

**One body at a time, and that does not change.**
A body is a function: its parameters, its result, and the entities it declares inside itself.
The unit of checking is the body, not the module
([ADR-0009][0009-pass-contract.md]'s sketch,
[ADR-0010][0010-stable-entity-identity.md]'s table),
and this is a property of the design rather than a step toward something bigger:
checking a body must not need the _result_ of checking another,
or the pass stops being a function of its own data and the surfaces it read,
and the bodies of a module stop being checked in parallel.

**Every signature of a top-level declaration is written, for now.**
The language lets a private function omit its types
([ADR-0004][0004-module-system.md] requires them of a public one);
the first checker does not read one without them.
Inferring the signature of a top-level function would mean checking one body against another,
which needs an order between the bodies of a module --- a dependency graph, or a fixpoint ---
and that is deferred.
A missing annotation, and a `_` where a type is required, are reported as mistakes,
and the incomplete declaration gets `Ty::Error` in place of what it does not write,
so its body still checks and reports its own mistakes once.
Inference of a top-level signature is not refused forever:
it returns when there is an order to check the bodies in,
and what it changes then is the rules of the checker, never the shape of a `Ty`.

**Signatures are resolved before any body is checked.**
The types a module's declarations write are resolved by a pass of their own over the item tree
(`resolve_module_types`),
which reads the names the resolution produced and never a body;
`ModuleTypes` is that pass's value, and it is what both the module's readers
and the module's own body checks read.
The pass is not inference:
every type is written, and resolving one is following the path to its class and its arguments,
walking the modules it names over the closure like any other path.

**Bidirectional, with one unification.**
`infer(expr) -> Ty` computes a type where nothing fixes it;
`check(expr, expected)` verifies one where something does ---
a literal against `Int`, an argument of a call against the parameter type,
the body of a declaration against its written result.
A written type is checked, not only unified,
which is what makes an annotation improve the error it is at.

**Level-based generalization.**
A `let` owns the type variables created while its right side is inferred;
unification lowers the level of a variable that meets an older one,
and at the end of the `let` a variable still owned by it becomes a `Param`
of the entity whose check generalized it.
This is the algorithm Rémy discovered, as the note [okmij] explains:
generalization is a walk of the type and nothing else,
no environment is scanned, and no scope check is written.
The eager form --- occurs check and level update on every unification ---
is enough for the first checker;
the note's lazy form is an optimization that changes nothing that is stored.
Generalization is where a `Param` is born today:
a local `fun identity(value) = value` is stored with one parameter standing for both occurrences of
its variable --- `(a) -> a` ---
and a call of it replaces the parameter with a fresh variable,
instantiating both occurrences together.
A top-level function will have its parameters declared rather than inferred
when the syntax for declaring one exists;
the storage is the same.

**Nothing of the checker leaks.**
Every variable that reaches storage is either resolved or generalized into a `Param`;
the pass is written so that storage is reachable only through a step that does this.
`Ty::Error` absorbs what the checker cannot make sense of,
so a broken expression reports once and does not cascade.
A `TypeDiag` names the place in the HIR ---
an `ExprId`, a `PatId`, or an entity and the place of a type in it ---
and the types it is about,
so the driver renders it with the source map it already holds,
and a type is rendered by a function next to `Ty` itself,
from the names inside its `EntityLoc`s, with no store to consult.
A check walks the paths of a body, which a resolution never reads,
so the check is the first to report what those paths name ---
and a name the walk could not find may be a name a module keeps to itself:
the rendering takes the same look a resolution's rendering takes (`hidden_name`),
and reports it as kept.

**The builtins are a table, not a type.**
`Int`, `Unit`, `String`, and `Bool` are ordinary classes;
the operators of the HIR and the literals are given types by a table inside the checker,
which is where the first `impl`s will replace it.
No type of the language is special in the representation.
The classes the table holds are the ones the standard library declares with `#[builtin]`:
the driver reads them off `std::core` and hands them over as `Builtins`,
which holds the four and nothing else ---
not an `Option`, because a caller that checks a body always has them.
A driver whose host recorded no library holds no classes and checks no types:
what a literal is is nothing the checker can say.

**Explicitly temporary.**
The algorithm, the tables, the wording of the diagnostics, and the crate that holds them
(`mlkc-typeck`) may be replaced whole.
What may not change is the values of `mlkc-hir-ty` and the rules above:
they are the contract between checking and everything after it.
A dump of a checked body, and of a module's types, is what the snapshot tests of
[ADR-0006][0006-snapshot-testing.md] compare,
and the dump is a function of those values alone.

### What this record does not decide

- The syntax that declares type parameters, and the arity rules of classes.
  A class declares no parameter today;
  an argument written at one is read as a type and reported where the arity is read.
- The inference of the signature of a top-level function.
  It waits for an order between the bodies of a module --- a dependency graph, or a fixpoint ---
  and until it exists every top-level declaration writes its type,
  and a `_` in one is a mistake like an omitted type.
- How an inferred parameter is named when it belongs to a binding inside a body
  rather than to a declaration.
  Today the checker names it after the entity whose check generalized it;
  a declared variable, when the syntax for one arrives, is named by its declaration.
- Fields, variants, and the layout of a value: the first checker types what the language has.
- `impl` resolution, coherence, and the orphan rules.
  They are the subject of [ADR-0004][0004-module-system.md];
  what this record fixes is that an `impl` answers a question of the checker
  and never adds a variant to `Ty`.
- The value restriction:
  the language has no effects yet, so every `let` generalizes.
  Effects will narrow what is generalized, not how a generalized variable is stored.
- Recursive and aliased types:
  the representation is a finite tree,
  and an alias that expands to itself would need more than this record designs.

### Positive Consequences

- A type is a value:
  the driver retains, compares, and back-dates it like every other value
  ([ADR-0008][0008-compiler-driver.md]).
- A body edit cannot change what its module shows:
  every signature is written, and a surface is resolved from signatures alone,
  so an edit inside a body invalidates the check of that body and nothing that reads the module.
- A check is retained like every other value ([ADR-0008][0008-compiler-driver.md]):
  the driver keys it by the body, the resolution, the closure, and the surfaces it read,
  and a check that recomputes to the same value is the value the driver held,
  so a body edit moves neither the bodies that did not change
  nor the diagnostics rendered from them.
- Bodies are independent units of work:
  a check reads its own body, the surface of its module, and the surfaces of the modules it names,
  so the bodies of a module are checked in parallel the way the modules of a project are
  ([ADR-0008][0008-compiler-driver.md]).
- A type read from another module is meaningful on its own:
  no reader keeps the other module's arenas alive,
  and a diagnostic carries a type to a host that never saw the checker.
- The checker is replaceable:
  improving it, or throwing it away for a constraint solver or an `impl`-driven one,
  touches nothing that stores a type.
- The classes of the language are declarations, not a primitive table:
  `Int`, `Unit`, `String`, and `Bool` are classes the standard library declares `#[builtin]`,
  and the compiler carries no primitive-type table in its representation.
- Renaming and inserting entities move no type
  ([ADR-0010][0010-stable-entity-identity.md]):
  a type keys on names, and a name is what a revision preserves.

### Negative Consequences

- Equality and storage are structural:
  two equal types built twice are two allocations,
  and comparing deep types costs a walk; there is no O(1) type id to compare.
- Every top-level declaration writes its types for now:
  a private helper whose signature the language could infer is a mistake the checker reports,
  and the restriction stays until the bodies of a module have an order to be checked in.
- The surface is one value:
  a change to any entry changes the value,
  so an edit that adds a function re-runs readers that never named it,
  until per-entity keys arrive ([ADR-0008][0008-compiler-driver.md] defers them).
- The first checker is a promise to be rewritten:
  it will have no user-written generics, no `impl`s, and not the diagnostics the language
  deserves, and code that grows around its limits is code to revisit.
- A driver whose host recorded no standard library checks no types:
  the classes of the language are what the library declares,
  so there is nothing for a literal or an operator to be.
- The representation is a commitment:
  a feature it cannot express as new variants and rules supersedes this record.

## Pros and Cons of the Options

### Types as self-contained values

Chosen. A `Ty` is a recursive enum that owns its children and names its classes by `EntityLoc`.

- Good, because a type survives the pass that made it:
  the driver compares and back-dates it, a diagnostic carries it, and it serializes.
- Good, because a type that crosses a module boundary is exactly the value its writer meant:
  no arena of another module is retained, and no id of another module is interpreted
  ([ADR-0010][0010-stable-entity-identity.md]).
- Good, because the enum has no variant for an inference variable,
  so what must not be stored cannot be stored.
- Good, because a feature extends the enum with a variant or a rule
  while the storage shape stays.
- Bad, because equality and copying are not O(1), and two equal types share nothing.
- Bad, because a recursive alias cannot be stored as a finite tree;
  the representation is not universal.

### Types as positional ids into a per-module type store

The design of [ADR-0003][0003-id-based-ir.md] applied to types: a flattened arena per checked unit,
and `TyId = Idx<Ty>` where a type is a node of that arena.

- Good, because it follows the IR principles of the project:
  dense storage, 32-bit handles, cheap allocation and traversal.
- Good, because a check can index and rewrite its temporary structures without naming anything.
- Bad, because a type in a surface would be a foreign id:
  a reader would have to retain the writer's store to interpret it,
  and ids of two stores are not comparable ---
  exactly what [ADR-0010][0010-stable-entity-identity.md] rules out for anything that crosses a
  revision.
- Bad, because a type in a diagnostic, a saved surface, or a test fixture
  would drag its store with it.
- Bad, because the driver's comparison of two revisions would be a walk over two arenas,
  not a comparison of values ([ADR-0008][0008-compiler-driver.md]).

### Types as handles of one global interner

Hash-consing: `Ty` is interned once per structural value, and two equal types are one pointer.

- Good, because equality is a pointer comparison, sharing is automatic,
  and a type is a compact key.
- Bad, because a global mutable table violates the pass contract
  ([ADR-0009][0009-pass-contract.md]): a pass would read and write state of its own.
- Bad, because an id depends on interning order and on what is still alive,
  which the determinism rule of [ADR-0009][0009-pass-contract.md] forbids
  and the on-disk cache of [ADR-0008][0008-compiler-driver.md] cannot rely on.
- Bad, because retention becomes a question of its own:
  the interner of [ADR-0010][0010-stable-entity-identity.md]'s names works
  because a name is a leaf that the input itself pins,
  and a type is a tree that nothing else pins.

### No resolved language

`TypeRef` stays the only representation, and each consumer resolves what it needs.

- Good, because there is nothing to design now.
- Bad, because every consumer --- MIR, the IDE, the crossing of a module boundary ---
  would resolve with rules of its own, and the resolved forms would drift.
- Bad, because the result of inference has no home:
  the type of an expression is not written anywhere at all.
- Bad, because comparing two signatures would compare written paths,
  which are not canonical and are not types.

### Bidirectional Hindley–Milner, one body at a time

Chosen for the first checker. `infer` and `check` share one unification;
generalization is level-based ([okmij]); a top-level signature is written, and a body is checked on
its own.

- Good, because the language writes types on declarations,
  so checking one is a first-class operation,
  and a mistake is reported where the annotation stands.
- Good, because levels make generalization cheap and scoped,
  and the algorithm is small enough to be thrown away.
- Good, because a check reads only its own body and the surfaces it names,
  so the bodies of a module are independent units of work and of invalidation.
- Good, because it is standard: the references are public,
  and the implementation needs no record of its own.
- Bad, because inference is only as good as the annotations around it,
  and a top-level function that could be inferred must write its type for now.
- Bad, because pure Hindley–Milner has no answer for the features to come ---
  `impl`s, overloaded operators, higher-rank polymorphism ---
  and is a placeholder for a checker that will.

### One check per module

Check the module as one unit, so a signature may be inferred from a body
and a body may use a function whose signature is inferred.

- Good, because the language lets a private function omit its types
  ([ADR-0004][0004-module-system.md]), and one context could infer them all.
- Good, because a call between two bodies of the module needs no surface to be complete first.
- Bad, because a body check would depend on other bodies of the module,
  so an edit inside one would re-check them all.
- Bad, because the unit of parallelism and invalidation would be the module, not the function,
  against the granularity of [ADR-0008][0008-compiler-driver.md].
- Bad, because the table of [ADR-0010][0010-stable-entity-identity.md]
  names a check per `BodyEntityLoc`, and a module-sized check throws that granularity away.

### Algorithm W

Inference only: an annotation is one more type to unify.

- Good, because it is the classic algorithm, and one function is the whole checker.
- Bad, because a written type would not propagate into an expression,
  so a literal, a call, and a future function value would be inferred blind and then compared.
- Bad, because the better error an annotation should give is exactly what it does not produce.

### No checker yet

Wait until the language settles, then write the real one.

- Good, because no work is thrown away.
- Bad, because the back end, the IDE's hover, and every consumer of a cross-module signature
  stay blocked on a language that is years from settled,
  which is the opposite of the plan of [ADR-0008][0008-compiler-driver.md].

## Links

- Pipeline and the type checking stage: [0005-compiler-pipeline.md]
- Module rules the surfaces are keyed by: [0004-module-system.md]
- The driver that retains and compares values: [0008-compiler-driver.md]
- The contract of a pass, inputs, diagnostics, and determinism: [0009-pass-contract.md]
- Two kinds of handle, names, and what may cross a revision: [0010-stable-entity-identity.md]
- Interfaces, closures, and what a resolution reads: [0016-inter-module-resolution.md]
- The arena model this record does not apply to types: [0003-id-based-ir.md]
- Snapshot tests of a checked module: [0006-snapshot-testing.md]
- The snapshots that hold the dumps of these values: [project_spec.rs]
- Rémy's level-based generalization, explained by Kiselyov: <https://okmij.org/ftp/ML/generalization.html>
- Rémy's technical report: <http://gallium.inria.fr/~remy/ftp/eq-theory-on-types.pdf>
- Bidirectional typing: <https://arxiv.org/abs/1908.05839>
- rust-analyzer's resolved types: <https://github.com/rust-lang/rust-analyzer/tree/master/crates/hir-ty>
- Implementation: `mlkc-hir-ty` (the values), `mlkc-typeck` (the temporary pass),
  and the slots of [mlkc-driver] that hold them and render their diagnostics

[0003-id-based-ir.md]: 0003-id-based-ir.md
[0004-module-system.md]: 0004-module-system.md
[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0006-snapshot-testing.md]: 0006-snapshot-testing.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0010-stable-entity-identity.md]: 0010-stable-entity-identity.md
[0016-inter-module-resolution.md]: 0016-inter-module-resolution.md
[okmij]: https://okmij.org/ftp/ML/generalization.html
[mlkc-hir-def]: ../../crates/mlkc-hir-def
[mlkc-driver]: ../../crates/mlkc-driver
[project_spec.rs]: ../../crates/mlkc-driver/tests/project_spec.rs
