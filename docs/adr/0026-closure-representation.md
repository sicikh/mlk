# Represent a lambda as a lifted function and a closure word

- Status: accepted
- Date: 2026-10-04

## Context and Problem Statement

A lambda is a value the language has as far as the checker.
The HIR holds `Expr::Lambda` with its parameters, its body, and the bindings its body reads,
which the free-variable analysis works out ([`mlkc-hir-def`]);
the checker gives it a `Ty::Fn` ([`mlkc-typeck`]);
and MIR reserved `Callee::Indirect` for the day functions become values ([ADR-0019][0019-mir.md]).

Two stages stood between that and a value a machine holds:
MIR, which held no construct that makes one,
and the WASM back end, where an indirect call was a `CodegenDiag::Unsupported`
"until closures exist" ([ADR-0020][0020-wasm-backend.md]).

The word model fixes the outline already:
a closure is a reference to a GC struct wrapping a funcref and the environment
([ADR-0018][0018-values-as-words.md]),
and the GC types of a module are structural,
so two modules that declare one type declare it byte for byte ([ADR-0021][0021-translation-units.md]).

What is left to decide is the shape of that struct and the code it carries,
the way the captures cross into that code,
the way a call goes through it,
and what the uniform word ABI makes of the polymorphism a `let` gives a lambda.

## Decision Drivers

- **One representation of a function value.**
  Two closures of one type must join at a control-flow join,
  be held in one local, and — when a signature can write a function type — cross a module boundary
  without a cast the source type cannot justify.
- **Types are not erased** ([ADR-0018][0018-values-as-words.md]).
  The code is named by a function type and the environment by its fields;
  nothing is a bag of bits walked at run time.
- **Separate compilation.**
  Every type a closure uses must be a function of the checked types,
  so two modules that never met declare it identically ([ADR-0021][0021-translation-units.md]).
- **Incrementality.**
  A lambda is inside the body that writes it;
  editing one rebuilds that body and nothing else ([ADR-0003][0003-id-based-ir.md], [ADR-0008][0008-compiler-driver.md]).
- **The pass contract.**
  Every stage this record adds is a pure, total, deterministic function ([ADR-0009][0009-pass-contract.md]).
- **Debug information.**
  A stack trace and a debugger must name the code of a lambda
  and show the values it captured ([ADR-0023][0023-debug-information.md]).
- **No mutation in the language.**
  A captured binding is a value; nothing writes back, so nothing needs a box.
- **A capturing closure is one object.**
  A function value is an ordinary value of the language,
  and the common case should cost what a structure costs:
  a closure that captures is one allocation,
  and a call walks the value it was handed and no second object.
- **The target is WASM GC with typed function references.**
  No tables, no runtime assignment of indices, no `call_indirect` over a table
  ([ADR-0020][0020-wasm-backend.md]).

## Considered Options

- **An environment that is the closure: the code in a supertype, the captures in a subtype.**
  The closure struct is a function of the function type and holds the code;
  an environment shape is a final subtype that adds the captures,
  and the closure is its own environment.
- **A uniform closure: a code reference and a separate environment struct.**
  The closure struct is a function of the function type;
  the captures live in a struct the closure points at.
- **A closure per lambda: the code and the captures in one struct, with no common supertype.**
  One struct per lambda; two closures of one signature have no type a local or a join can hold.
- **A uniform closure with an untyped `funcref`.**
  One closure struct type for every function type; the code is cast before every call.
- **No closure object: captures as arguments.**
  Nothing is allocated; a call passes the captures like other arguments.

## Decision Outcome

Chosen option: "An environment that is the closure: the code in a supertype, the captures in a subtype",
because it is the only one where the type of a function value is a function of the checked type
and a capturing closure is nevertheless one object:
a closure of one signature is one WASM type whatever the lambda captured,
so the closure of one branch joins with the closure of another,
a local of a function type has one type on every path,
and a future function type in a signature names one structurally canonical type;
and the environment a lifted function reads is the closure value itself,
so creating a closure is one allocation and calling it fetches the code from the value
the call already holds.

### A lambda is a lifted function of the module

A lambda becomes a _lifted function_: a body of its own,
next to the body that wrote it in the MIR of one HIR body ([ADR-0019][0019-mir.md]).
The HIR keeps the lambda in the body that wrote it, because a HIR body is the unit of
incrementality ([ADR-0003][0003-id-based-ir.md]);
MIR lifts it, so that nothing after MIR reads a nesting.

```rust
// mlkc-mir
/// The id of a lambda in the HIR body that wrote it.
///
/// A lambda has no name and no identity that outlives the body it is written in: the number
/// is its place in the order the lowering wrote it, and the HIR body is the rest of its
/// identity ([ADR-0003][0003-id-based-ir.md]).
pub struct LambdaId(u32);

/// Where a function of MIR is: the body of an entity of the project, or a function lifted
/// out of one --- a function declared in a `local`, or a lambda ([ADR-0019][0019-mir.md]).
pub enum FunctionLoc {
    Entity(BodyEntityLoc),
    Lifted { origin: BodyEntityLoc, id: LiftedId },
}

/// What a lifted function is inside the body that declared it.
pub enum LiftedId {
    /// A function declared in a `local`, by its place in the arena of the HIR body.
    Local(LocalFunctionId),
    /// A lambda written in the HIR body.
    Lambda(LambdaId),
}

/// The MIR of one HIR body: every function it declares, each a body of its own.
pub struct Bodies {
    /// The bodies: the body of the entity first, then the lifted ones.
    pub bodies: Vec<Arc<Body>>,
}

pub struct Body {
    /// The function the body is.
    pub function: FunctionLoc,
    /// The type the checker gave the lambda:
    /// what a closure of it takes and gives back.
    pub ty: Ty,
    /// The bindings the lambda captured, in the free-variable order of the HIR.
    pub captures: Vec<CaptureData>,
    /// The parameters, in the order they are declared.
    pub params: Vec<ValueId>,
    pub entry: BlockId,
    pub blocks: Arena<Block>,
    pub values: Arena<ValueData>,
    pub locals: Arena<LocalData>,
}

/// One binding a lambda captured.
pub struct CaptureData {
    /// The name the binding was written under, if it was.
    pub name: Option<Name>,
    /// The type the checker gave the binding.
    pub ty: Ty,
    /// Where the binding is written, for the debug tables.
    pub span: Span,
}
```

`Bodies` is the value of one HIR body:
the body of the entity first,
then the functions declared in a `local` in declaration order,
then the lambdas in the order the lowering wrote them.
A lifted function is named inside its module by its `FunctionLoc`:
`fib`, `fib::aux`, `fib::<mlkc@lambda-0>`.

`Rvalue` gains two variants, and `Callee::Indirect` finally means what it was reserved for:

```rust
pub enum Rvalue {
    // ... the variants of [ADR-0019][0019-mir.md], and:
    /// A closure: the lambda that is its code, and the words it captures.
    Closure {
        /// The code.
        lambda: LambdaId,
        /// The captured words, one per capture of the lambda, in the same order.
        captures: Vec<Operand>,
    },
    /// The value of one binding the enclosing lambda captured; only in a lambda body.
    Capture {
        /// Which capture, by its place in `Body::captures`.
        index: u32,
    },
}
```

- **The lowering.** The functions a body declares are lifted as the body lowers:
  a function declared in a `local` or a lambda becomes a body of its own,
  and the value of one HIR body is the flat `Bodies` of them.
  A lambda expression lowers to `Closure`,
  whose capture operands are the places the captured patterns are bound to in the enclosing
  frame, read in the order the free-variable analysis gave,
  and whose `lambda` is the lifted body in the same set.
  Inside the lambda body a parameter lowers to its parameter value,
  a path anchored to a captured binding lowers to `Rvalue::Capture`,
  bound to a slot by the first statement of the entry block,
  and everything else lowers as it does in a body.
- **A nested lambda captures from its own body.**
  A lambda written inside a lambda captures the parameters and captures of the lambda around it
  by the same rule; the word it captures is a value of that frame,
  and the outer environment is not reachable from the inner code.
- **A capture is the word, by value.**
  The environment is filled when the closure is created,
  in the order the free-variable analysis gave,
  from the places the captured bindings hold in the enclosing frame.
  An immediate is copied; a captured heap object is the reference it always was,
  and the GC keeps it alive as long as a closure that captured it is reachable.
  Nothing is ever written back — the language has no mutation —
  so an environment is immutable and no capture needs a cell.
  A later binding of the same name shadows the name and not the capture:
  the closure keeps the binding it captured.
- **A call through a closure passes the source-level arguments only.**
  The captures are inside the closure, not in the argument list;
  `Callee::Indirect` is the operand holding the closure,
  the environment argument the back end adds is that same value,
  and the lowering records the call's type from the checker as it does for any call.
- **`ty` is the checker's type, and not the generalized one.**
  `Body::ty` is `CheckedBody`'s type of the lambda expression,
  so an unconstrained parameter is `Ty::Error` and a constrained one is its class;
  what the checker generalized at the `let` is the type of a _use_, not of this code
  (see "The key is the ABI shape").
- **The verifier and the dump.** `Capture` is valid only in a lifted lambda body,
  and its index is in range for that body's captures;
  `Closure` passes exactly one operand per captured binding;
  each lifted function is verified as the body it is, with the SSA invariant of its form.

#### Why every lifted function is a body of its own

The blocks, values, and slots of a lifted function are its own arenas,
separate from the arenas of the body that wrote it,
because they mirror the partition the machine has:
a lambda becomes a WASM function, and every stage that works on code is stated per function.
The SSA construction computes one dominator tree from one entry;
the verifier checks one value space and one slot space;
the local allocator of the back end numbers the locals of one function;
the LIR lowering maps one body to one function.
One `ValueId` space shared with the writer would hold two functions,
and every pass would grow a partition it does not have
— which function of this arena is this block in —
that one body per function states for free.

MIR states that shape flat: `Bodies` holds the body of the entity and every function lifted
out of it, and nothing after MIR reads a nesting, because there is none ([ADR-0019][0019-mir.md]).
The writer is still the only identity a lifted function has in the driver:
a lambda has no name of its own and no edit changes it alone;
it is rebuilt exactly when the HIR body that wrote it is,
so the HIR body remains the unit of invalidation and of memoization, as it already is,
and the driver's keys do not grow a granularity that
[ADR-0008][0008-compiler-driver.md] defers.
In MIR a lifted function is addressed by its `FunctionLoc`, whose origin is that HIR body,
and the `LambdaId` in it is a place in the order the lowering wrote,
which [ADR-0003][0003-id-based-ir.md] does not promise across rebuilds
— so the location is part of the value, and not a key of the driver.
A lambda is moreover reachable only through the closure its writer creates,
so nothing can pull its code without pulling the writer anyway.

The consequence is deliberate:
the per-body stages (the lowering, `mir-ssa`, and `lir`) are keyed and guarded per HIR body,
and the value one of them produces holds the corresponding form
of the writer and of every function lifted out of it.
The pass lifts the functions inside the one guarded call,
and a bug in a lambda's code is a bug of the writer's body,
which is the finest identity the driver has for that code.
If a later record narrows the driver's units,
a lifted function can be addressed by its `FunctionLoc`, which MIR already names it by,
so nothing outside the writer needs to change.

The driver's stages stay per HIR body;
the SSA construction, the verifier, and the dumps read each function of the value they are
handed, and nothing else changes.
A call written inside a lambda is a call of the module like any other:
the stage that collects the callees of a module reads every function, lifted or not,
and imports what they name.

### The WASM closure

A function value of checked type `P -> R` has types of two canonical kinds;
`P -> R` names the source type, and what keys them is its ABI shape:

```text
(rec
  (type $fn(P -> R)      (func (param (ref $closure(P -> R))) <abi P>... (result <abi R)))
  (type $closure(P -> R) (sub (struct (field (ref $fn(P -> R))))))
)
(rec
  (type $env(P -> R; C0, C1, ...) (sub final $closure(P -> R)
    (struct (field (ref $fn(P -> R))) (field <abi C0>) (field <abi C1>) ...)))
)
```

- **The closure is one open struct per function type, and it holds the code.**
  `$closure(P -> R)` is the type of every function value of the shape, whatever it captured;
  its one field is the code, a typed function reference `(ref $fn(P -> R))`, never null.
  It is declared with `sub` and no supertype, so every environment of the shape can be its
  subtype, and it is instantiable itself:
  the closure of a lambda that captured nothing is an instance of it.
- **The environment is a final subtype that adds the captures.**
  A capture shape `C0, C1, ...` is one `$env(P -> R; C0, C1, ...)`,
  declared `sub final $closure(P -> R)`;
  its fields repeat the code as the prefix a subtype must repeat,
  then the captured words in the order of `Body::captures`,
  typed by the ABI of their checked types.
- **The lifted function takes the closure first.**
  `$fn(P -> R)` is the signature of every lambda of that shape:
  a `(ref $closure(P -> R))`, then the declared parameters in the ABI of
  [ADR-0021][0021-translation-units.md],
  and the result in the same ABI.
  A parameter or result of an immediate type is an `(ref i31)`;
  everything else is a word, an `eqref`.
  The environment a lambda reads is that first parameter itself,
  cast to the environment type its code was written for:
  field 0 of the cast is the code, and the captures follow.

`$fn` and `$closure` are mutually recursive, so they are one recursion group,
declared once per shape and containing exactly those two types:
two modules that use one function type declare the same group, byte for byte.
An environment is a group of its own, and that is a rule, not a convenience:
a recursion group is part of the identity of the types it defines
([ADR-0021][0021-translation-units.md]),
so an environment declared inside the shape's group would make the group's identity depend
on the captures,
and two modules that use one function type would declare different `$fn`s
as soon as their lambdas capture different shapes.
The environment group follows the shape group and names its supertype in it;
a `sub` may reference a type of an earlier group, which is what lets the two live apart.
Nothing references a `$closure` except its own group's `$fn` and the environments of its shape.
A lambda cannot capture itself,
because the lowering resolves the name of a binding only after the expression it is bound to,
so `let f = fn(x) -> f(x)` does not resolve `f` ([ADR-0004][0004-module-system.md]);
every environment is filled with values that already exist when the closure is created.

#### The key is the ABI shape, not the type

The checked type of a closure is not always concrete.
`let id = fn(x) -> x in id(1)` with `id(true)` elsewhere gives the binding the generalized
`Ty::Fn` with a `Ty::Param`, while the lambda expression itself is recorded with `Ty::Error`
for the same variable;
two closures of one signature must still be one WASM type.

They are, because the key of a closure type is the **shape** of the checked type:
whether a parameter or result is an immediate is what the ABI cares about,
and every other type — a class, a `Param`, an `Error` — is one word.
`Ty::Param` and `Ty::Error` are words,
so both spellings of `id` meet in one `$fn`, one `$closure`,
and one `$env` per capture shape,
and the closure a `let` generalizes is stored, joined, and called as one value.
The key of an environment type is the same shape plus the ABI shape of each capture,
so a lambda that captured nothing has no environment type at all:
it is the closure type itself.

This is where the uniform word earns its keep:
the lifted code of a generalized lambda is monomorphic over words,
and each instantiation of the `let` only chooses the values that flow through it.
An operation inside the lambda that needs a class (`+` on an `Int`) has that class in the
checker's recorded type, so it is no longer a `Param`;
an operation that is only passing a value around needs nothing but a word.
The backend refines and casts at the uses, as it does everywhere else
([ADR-0020][0020-wasm-backend.md]).

#### Creation, reading a capture, and the call

Creating a closure that captures `c0, c1` in the enclosing function:

```text
ref.func $f                                    ;; (ref $fn(P -> R)): the code, typed
struct.new $env(P -> R; C) (ref.func $f) (c0) (c1)
```

The result is an environment, and an environment is already a closure:
where the closure type is expected — a local, a block parameter, a join —
the value needs no instruction, because the subtyping is declared.
A lambda that captures nothing creates the supertype itself, with one allocation as well:

```text
struct.new $closure(P -> R) (ref.func $f)
```

The entry of a lifted function reads its captures:

```text
local.get $closure                    ;; (ref $closure(P -> R)): the first parameter
ref.cast (ref $env(P -> R; C))        ;; never traps: the code and the closure agree
struct.get $env(P -> R; C) 1          ;; capture 0; field 0 is the code
struct.get $env(P -> R; C) 2          ;; capture 1
```

A call of a closure `c` of checked type `P -> R`:

```text
ref.cast (ref $closure(P -> R)) c     ;; only where c is known to be a word alone
local.get $c                          ;; the environment argument: the closure itself
<arguments>                           ;; the declared parameters, in their ABI
struct.get $closure(P -> R) 0 (local.get $c)  ;; the code
call_ref $fn(P -> R)
```

`call_ref` is the typed call of the function references proposal:
the signature is an immediate of the instruction, taken from the checked type of the callee,
and the code field is typed by the same `$fn`, so no table and no dynamic signature test exist.
The closure is read twice — once to be passed as the environment, once for its code —
and both reads are of the value the call already holds, not of a second object.

`ref.func` requires the function to be declared,
so the back end declares every lifted function it references in a declarative element segment.

### The back end: refinement, LIR, and layout

The refinement lattice of [ADR-0020][0020-wasm-backend.md] gains one entry:

```text
Closure(P -> R)   -- a value known to be a closure of this signature
```

It is the result of `Rvalue::Closure`, and it is what a local of a function type is typed with
when its definition is known;
`P -> R` is the ABI shape of the signature, as in the types above,
so `Closure(x)` joins with `Closure(x)` to `Closure(x)` and with anything else to `Word`.
The WASM type behind it is `(ref $closure(P -> R))`, the type that holds any environment of
the shape;
an environment whose shape the lowering knows keeps its `(ref $env(...))`
and is subsumed where the closure is expected.
A word that is only a word is cast where it is called,
exactly as a structure is cast where a field is read.

The WASM LIR of [ADR-0022][0022-wasm-lir.md] names the concrete types and the instructions:

```rust
// mlkc-lir-wasm
pub enum RefTy {
    I31,
    Eq,
    /// A concrete GC type of the module: a struct or an array, by its type index.
    Type(u32),
}

pub enum Op {
    // ... the instructions of [ADR-0022][0022-wasm-lir.md], and:
    /// `ref.func`: a function of the module, as a reference.
    RefFunc {
        function: FuncIndex,
    },
    /// `ref.null`: the null of a reference type.
    RefNull(RefTy),
    /// `struct.new $t`.
    StructNew {
        ty: u32,
        fields: Vec<ValueId>,
    },
    /// `struct.get $t $i`.
    StructGet {
        ty: u32,
        field: u32,
        value: ValueId,
    },
    /// `call_ref $sig`: the call of the function a value holds.
    CallRef {
        signature: u32,
        callee: ValueId,
        args: Vec<ValueId>,
    },
}
```

`Op::CallIndirect` was the placeholder of this record and is replaced by `CallRef`:
selection reads the code from the closure's first field and passes the closure itself
as the environment argument, so the call itself is one typed call of a value.
`StructNew` and `StructGet` are the target's own struct instructions,
which the structures and strings of the language will use as they are lowered;
this record uses them for the closure: one `struct.new` builds it,
and `struct.get` reads its code and its captures.

The module's layout numbers the functions and declares the types ([ADR-0019][0019-mir.md]):

```rust
// mlkc-codegen-wasm
/// One function of a module, as the back end reads it.
pub struct ModuleFunction {
    /// The body of an entity, a function declared in a `local`, or a lambda.
    pub function: FunctionLoc,
    /// The name the function is called by: `fib`, `fib::aux`, `fib::<mlkc@lambda-0>`.
    pub name: String,
    /// What the function takes and gives back, without an environment.
    pub signature: FnSignature,
    /// Whether the module exports it; only a function of an entity is ever exported.
    pub exported: bool,
    /// The SSA body of the function.
    pub body: Arc<MirBody>,
}

/// What the layout reserves for one lifted lambda.
pub struct ClosurePlan {
    /// The index of the lifted function in the function index space.
    pub index: u32,
    /// The shapes of its captures, in the order they are stored.
    pub captures: Vec<AbiType>,
    /// The `$fn` and `$closure` types of its shape.
    pub types: ClosureTypes,
    /// The type of its environment, where it captured something.
    pub env: Option<u32>,
}
```

- Imports keep their order, then the functions the module declares in the order the module
  numbers them: the HIR bodies in module order, and the body of each entity followed by the
  functions lifted out of it, in the order `Bodies` lists them.
  A function's index therefore depends only on the module.
- The types of the closure family are collected from the lambdas the layout walks and
  deduplicated by key: one shape group per ABI shape, and one environment group per capture
  shape.
  The shape groups come first, then the environment groups, each naming the shape group it
  subtypes.
  The grouping is a rule and not a choice of the assembler:
  an environment declared inside the shape's group would change the shape's identity with the
  captures, and two modules would stop sharing `$fn`.
- A function lifted out of a body is never imported and never exported;
  a lambda is reached through `ref.func` in the closure that wraps it.
- The `lir` stage stays keyed by the HIR body:
  its value is the LIR of the owner's function and of every function lifted out of it,
  flat, in the shape of the MIR it read:

    ```rust
    // mlkc-codegen-wasm
    pub struct LoweredFunction {
        /// What the function is.
        pub function: FunctionLoc,
        /// The name the function is called by.
        pub name: String,
        /// The LIR of the function.
        pub body: Body,
    }

    /// The LIR of one HIR body: every function it declares, flat.
    pub struct LoweredFunctions {
        pub functions: Vec<Arc<LoweredFunction>>,
    }
    ```

    Encoding reads that value flat, in the order the layout numbers the functions;
    encoding a lifted function is encoding a body;
    `emit_function` and `assemble_module` learn nothing about closures
    beyond the instructions they already encode.

### Names and debug information

The name of a lifted function is derived from the body that wrote it and what it is there,
because a lambda has no name of its own and a stack trace still needs one;
a function declared in a `local` keeps the name it was declared under:

```text
fib::<mlkc@lambda-0>
fib::aux
```

The spelling is the one the HIR already uses for names no module can write
(`<mlkc@pipeline-0>`), so no source name collides with it and the owner is visible in the trace.

A lifted function is a function like any other for the debug tables
([ADR-0023][0023-debug-information.md]):
a subprogram named as above, with its low and high pc,
and what it captured and declares as locals with `DW_OP_WASM_location`.
The spans that line tables are made of are the spans the lowering copied into the lambda's
statements, so a stepper walks into a lambda as into any other body.

### Testing

- The MIR and the LIR of a body with a closure are snapshot-tested like every other value
  ([ADR-0006][0006-snapshot-testing.md]);
  the dumps spell the new nodes (`closure lambda#0 (capture slot#1)`, `capture #0`).
- The text form of a module (`wasmprinter`) is snapshotted for a program with a closure,
  so the recursion groups, the `sub` declarations, the `$fn`, `$env`, and `$closure` types,
  and the `call_ref` are reviewed.
- The type sections of two modules that use one function type but capture differently are
  compared, so the shape group is the same bytes in both and the environment groups differ
  ([ADR-0021][0021-translation-units.md]).
- Generated programs are executed in `wasmtime`:
  a lambda that captures a parameter;
  a captured `let` and a captured pipeline binding;
  a lambda inside a lambda;
  a lambda that captures another closure;
  a lambda that captures nothing;
  a lambda stored in a `let` and called at two types;
  two branches that produce closures of one signature and meet.
- Every emitted module is validated with `wasmparser`,
  with GC and function references enabled ([ADR-0020][0020-wasm-backend.md]).

### What this record does not decide

- **Function types in the source.** The language cannot write one yet,
  so a closure cannot cross a function boundary today;
  the representation is canonical already, and a future type syntax needs no new mechanism.
- **Deduplication and specialization.**
  A lambda that captures nothing still becomes a closure over the bare `$closure` type,
  and two identical lambdas are two functions;
  turning the first into a plain function and deduplicating the second are optimizations,
  to be decided when there is something to measure.
- **Recursion.** Self- and mutual recursion need a `letrec` and a knot in the environment;
  that is a language decision, not this representation's.
- **Mutable captures.** The language has no mutation;
  a mutable binding would need a cell in the environment, and would supersede this record.
- **Tail calls.** `return_call_ref` is an instruction this shape can use;
  the semantics and the stack guarantees of tail calls are a decision of their own.
- **Functions declared inside a body.**
  A function declared in a `local` is lifted exactly as a lambda is,
  under the same `FunctionLoc` with a different `LiftedId`,
  and differs from a lambda by having a name and a written signature;
  what it does not have is an environment: it reads nothing of the body that declares it,
  which is a rule of the language and not of this representation.
- **The interpreter.** It reads the same MIR construct —
  a closure is code plus captured words, and `Capture` indexes them —
  and its own value model is not this record's.
- **Host interop.** A closure is a word, and a host that has no word model sees it as an opaque
  reference; nothing here makes a closure callable from JavaScript.

### Positive Consequences

- A function value is a word like every other value,
  and the ABI never learns a second representation ([ADR-0018][0018-values-as-words.md]).
- The type of a closure depends on the checked type alone,
  so joins, locals, and — later — module boundaries need no casts that the source type cannot
  justify;
  the only subtyping of the representation is the declared environment-to-closure one.
- A closure that captures is one allocation, created by one `struct.new`,
  and a closure that captures nothing is one allocation of the supertype itself;
  no closure has a second object behind it.
- The environment is the closure, with typed fields:
  reading a capture is a `struct.get` on the parameter, not a dynamic lookup,
  and the debugger can name the values a lambda carries.
- The call is one `call_ref` over typed references whose code is one `struct.get`:
  no tables, no runtime signature tests, no dynamic dispatch of a language that has none.
- Nothing new is keyed by the driver:
  the body stays the unit of invalidation and of memoization ([ADR-0008][0008-compiler-driver.md]).

### Negative Consequences

- The type section needs recursion groups and declared subtyping,
  and the grouping is load-bearing:
  an environment declared in its shape's group still validates
  and only stops being shared across modules,
  so the rule lives in the assembler and in tests.
- The environment's key includes the function shape:
  two lambdas that capture the same shapes under two signatures declare two environment types,
  where an environment behind a pointer would share one.
- The entry of a capturing lambda casts its closure to the environment it was written for.
  The cast cannot fail, because the lowering creates the code and the environment together,
  but it is a subtype test and not a free instruction.
- The lifted function of a generalized lambda is one function over words:
  the casts a concrete instantiation would not need are emitted at the uses,
  as everywhere else the word model erases a type.
- A lambda becomes one more WASM function per expression,
  with a name and a debug entry of its own;
  a module of many small lambdas pays for the section entries.
- The static type of a function value says nothing about what it captured:
  a debugger shows the captures through the DIE of the lifted function,
  and a tool that inspects a closure word needs the code, or a cast to its environment type,
  to know the environment's shape.

## Pros and Cons of the Options

### An environment that is the closure: the code in a supertype, the captures in a subtype

Described above: `$closure(P -> R)` is an open struct holding `(ref $fn(P -> R))`,
`$env(P -> R; C...)` is a final subtype that repeats the code field and adds the captures,
and the lifted function takes the closure and casts it to its environment.

- Good, because the closure type is a function of the function type,
  so one signature has one representation on every path and in every module,
  and the environment does not need to be pointed at: it is the closure.
- Good, because the code field is typed:
  `call_ref` needs no cast where the refinement knows the closure,
  and a wrong signature is a validation error of the module, not a run-time test.
- Good, because a closure that captures is one allocation,
  and a closure that captures nothing is an instance of the supertype itself:
  no null environment and no special case at the call or at the entry.
- Good, because a call reads the code from the value it already holds and passes that value on;
  there is no second object to fetch.
- Good, because reading a capture is one `struct.get` on the first parameter.
- Bad, because the type section needs recursion groups and declared subtyping.
- Bad, because the environment's canonical key includes the function shape,
  so one capture shape has one environment type per signature rather than one in all.
- Bad, because the grouping is load-bearing and silently wrong if a writer puts it elsewhere:
  an environment in the shape's group still validates
  and only makes `$fn` differ between modules.
- Bad, because the entry of a capturing lambda casts its closure down to its environment,
  as any design in which the function type cannot name the captures must.

### A uniform closure: a code reference and a separate environment struct

`(struct (field (ref $fn)) (field eqref))` for every shape;
the captures live in the `$env` the second field points at.

- Good, because no recursion group and no declared subtyping is needed:
  every type is a group of one, and `$fn` does not mention the closure type at all.
- Good, because the environment type does not depend on the function type:
  one capture shape is one environment type in the whole program.
- Bad, because a closure with captures allocates twice: the environment and the closure.
- Bad, because a call fetches the environment from the closure and then the code from it:
  two `struct.get`s and two objects where the chosen design touches one.
- Bad, because the empty environment is a null special case:
  one closure type has two runtime shapes,
  and nothing may cast the second field before checking it.

### A closure per lambda: the code and the captures in one struct, with no common supertype

`(struct (field (ref $fn)) (field <abi C0>) ...)`, one type per lambda.

- Good, because a capture is read from the closure directly, with no second object.
- Good, because a closure with captures allocates once.
- Bad, because a function value has no type of its own:
  two lambdas of one signature have two struct types,
  and the GC proposal has no structural subtyping between declared types,
  so a local or a join would need a supertype nobody declared.
- Bad, because adding that supertype with a typed code field is the chosen option;
  without one, the common type must leave the code untyped,
  which reintroduces exactly the run-time signature test this record avoids.
- Bad, because a future function type in a signature can no longer name one canonical type;
  it would name a hierarchy of shapes.

### A uniform closure with an untyped `funcref`

One `(struct (field funcref) (field eqref))` for every function type.

- Good, because the type section is as small as it can be:
  one closure type, whatever the program writes.
- Good, because it works where typed function references are not available.
- Bad, because the signature of the code is not in the struct,
  so a call casts the code to the `$fn` the checker gave the callee before `call_ref`,
  at every call site, for no check the source did not already make.
- Bad, because the cast can trap where the typed field would have made the mistake a validation
  error of the module itself.
- Bad, because a wrong store into a closure could only be caught at a call.

### No closure object: captures as arguments

The code is called with the captures as leading arguments, and the closure value is only the
code (or the captures are threaded by hand).

- Good, because nothing is allocated for a closure.
- Bad, because the call site must know the captures of the callee,
  which a value of a function type does not say;
  the signature would have to include them, so the closure value would have to carry them anyway
  or the call would have to be untyped or defunctionalized.
- Bad, because a function value has two possible representations —
  a bare code reference when it captured nothing, and code plus captures otherwise —
  so its type at a join is neither.
- Bad, because defunctionalization turns every call into a dispatch over a closed set of
  lambdas, which cannot see a lambda of another module and cannot compile a polymorphic one.

## Links

- Values as words, and the closure row this record fills in: [0018-values-as-words.md]
- MIR, `Callee`, and the set of bodies a lifted function lives in: [0019-mir.md]
- The WASM back end, refinements, and the call it left unsupported: [0020-wasm-backend.md]
- Canonical types, the ABI, and separate compilation: [0021-translation-units.md]
- The LIR the new instructions belong to: [0022-wasm-lir.md]
- Debug information and what a subprogram is: [0023-debug-information.md]
- The body as the unit of invalidation: [0003-id-based-ir.md]
- The module system the self-capture argument uses: [0004-module-system.md]
- Snapshot testing: [0006-snapshot-testing.md]
- The driver whose stages stay per body: [0008-compiler-driver.md]
- The pass contract every new stage obeys: [0009-pass-contract.md]
- The HIR the lambdas were lowered into: [`mlkc-hir-def`]
- The checker that gives a lambda its type: [`mlkc-typeck`]
- The GC proposal, including the structural equality of declared types:
  <https://github.com/WebAssembly/gc>
- The function references proposal, and `call_ref`:
  <https://github.com/WebAssembly/function-references>
- The WebAssembly specification, for the final rules of GC and function references:
  <https://webassembly.github.io/spec/core/>
- Typed closure conversion, the design this representation follows:
  Morrisett and Harper, POPL 1996, <https://doi.org/10.1145/237721.237791>

[0003-id-based-ir.md]: 0003-id-based-ir.md
[0004-module-system.md]: 0004-module-system.md
[0006-snapshot-testing.md]: 0006-snapshot-testing.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0018-values-as-words.md]: 0018-values-as-words.md
[0019-mir.md]: 0019-mir.md
[0020-wasm-backend.md]: 0020-wasm-backend.md
[0021-translation-units.md]: 0021-translation-units.md
[0022-wasm-lir.md]: 0022-wasm-lir.md
[0023-debug-information.md]: 0023-debug-information.md
[`mlkc-hir-def`]: ../../crates/mlkc-hir-def
[`mlkc-typeck`]: ../../crates/mlkc-typeck
