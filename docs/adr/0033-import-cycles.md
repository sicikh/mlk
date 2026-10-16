# Find a cycle of module names in the resolution

- Status: accepted
- Date: 2026-10-16

## Context and Problem Statement

The imports of a program form no cycle ([ADR-0004][0004-module-system.md]):
a module is resolved, checked, and instantiated after the modules it names,
so a cycle of names is an order that does not exist,
and it is a mistake of the source rather than a state of the compiler.
[ADR-0008][0008-compiler-driver.md] says it plainly ---
"a cycle is a mistake of the source and never a state the driver handles" ---
and so does [ADR-0016][0016-inter-module-resolution.md].

Nothing reported one.
The walk of a resolution reports a _re-export chain_ that returns to itself,
which is a loop inside the walk of one module.
Two modules that name each other were no stage's to find:
they reached the link stage, where the modules of a program are ordered
for a host to instantiate ([ADR-0021][0021-translation-units.md]),
and the order found no order to give.
A program a person wrote was answered with an internal compiler error
at the assertion of the topological sort.

The question: what stage finds a cycle of module names,
what are the edges of the graph it is found in,
and what is a cycle to the stages after name resolution?

## Decision Drivers

- A mistake of the source is a diagnostic, never an ICE ([ADR-0009][0009-pass-contract.md]).
- A host that never builds a program sees it:
  the editor and the future LSP resolve names and stop there,
  so the report is a value of the resolution level and not of the link.
- The stages after the resolution rely on the order the names form:
  MIR, codegen, and the link are handed a program the front end read clean
  ([ADR-0008][0008-compiler-driver.md], [ADR-0021][0021-translation-units.md]).
- The pull graph stays stratified ([ADR-0008][0008-compiler-driver.md]):
  a cycle is found by a pass over values the driver already holds,
  not by a fixpoint and not by a query that runs into itself.
- Incrementality ([ADR-0004][0004-module-system.md]):
  an edit that does not move a name does not move the value the cycle was found in.
- A cycle is reported where it is written, and a reader of any of its files is told:
  the mistake is the source's, and the source is every module of the cycle.

## Considered Options

- **Find the cycle at the link stage** --- where the order is needed.
- **Find the cycle inside the resolution of a module, module by module** ---
  a resolution is handed the modules one module's paths reach, and a cycle is not one of them.
- **Find the cycle in a pass of the project, over the names of all its modules** --- the chosen option.
- **Report the cycle at one module of it**, or **at every module of it** --- the chosen form is every module.

## Decision Outcome

Chosen option: "Find the cycle in a pass of the project, over the names of all its modules",
and report it at every module of the cycle.

### The graph

The nodes are the modules of one project.
The edges say which modules the names a module writes belong to.

- **Every path a module writes is an edge**:
  the imports of its surface, the types its declarations write,
  and the paths of its bodies, which is the wider closure a check reads ([ADR-0017][0017-resolved-types.md]).
  A cycle closed by two calls through fully qualified paths, with no `use` at all, is a cycle
  like any other.
- **An edge points at the module a name belongs to**,
  and not at the module the path was written through:
  `use std::prelude::Int` reaches `std::core`, which is where `Int` is declared,
  and a re-export is transparent this way,
  which is also the module the back end imports the value from ([ADR-0021][0021-translation-units.md]).
- **A name a module writes of itself is not an edge**:
  what a module declares is read by its own bodies,
  and nothing has to stand before it.
- **An edge that leaves the project is dropped**:
  the projects a project names are the ones it depends on,
  and the graph of them is acyclic ([ADR-0008][0008-compiler-driver.md]),
  so a name of another project cannot come back.
  A module of no project names no module of its own, so it has no cycle to be in.

A cycle is then a strongly connected component of this graph:
the modules that reach each other.
A component of two modules or more is a cycle,
and the components are found in the order of the modules of the project.

### The passes

Two passes of the resolution level carry the graph, and the rendering reads them:

| Pass                     | Unit    | Key                                                                         | Value                                                 |
| ------------------------ | ------- | --------------------------------------------------------------------------- | ----------------------------------------------------- |
| `named_modules`          | module  | the HIR, the resolution, and the closure of a check                         | the modules the names reach, and where each is named  |
| `import_cycles`          | project | the `named_modules` of every module of the project, entry by entry          | the cycles, and one report per module of each of them |
| `resolution_diagnostics` | module  | the resolution, the HIR, the cycles of the project, and the HIR a look read | the diagnostics of the module, rendered               |

`named_modules` is a value of one module because that is what a cycle is found _from_:
an edit that moves no name of a module moves no value of it,
and a body edit that moves no name anywhere reads the graph of its project and no further.
`import_cycles` is a value of one project, keyed by the named modules of every module of it
the way the def map is keyed by resolutions ([ADR-0016][0016-inter-module-resolution.md]),
so a name that moves in one module recomputes the cycles of one project.

### The report

The report is `ResolveError::ImportCycle`, one per module of the cycle,
rendered into the resolution diagnostics of that module:

```
use project::other::g
    ^^^^^^^^^^^^^^^^^
the module `project::other` is in a cycle of names with this module:
the imports of a program form no cycle
```

Every module of a cycle is reported, and not the one "that closes" it:
a cycle of modules is closed by no single import ---
which one it would be is decided by where a reader begins ---
and the mistake is the source's rather than one module's ([ADR-0004][0004-module-system.md]).
A reader of any of the files of the cycle is pointed at a name that is in the cycle,
which is what an editor shows and what a person fixes.
The place is where the module named the reached module for the first time,
in the order the module is read in: an import, a declared type, or a path of a body.
A name the module did not write --- one the prelude brought in --- has no syntax to point at,
and the report is told without a place ([ADR-0011][0011-module-prelude.md]).

### What the link stage does

The link refuses to build a plan for a project whose closure holds a cycle:
`Driver::link` answers `None`, which is what it answers for any module the front end did not
read clean ([ADR-0021][0021-translation-units.md]).
It refuses by consulting the cycles the resolution found, and it finds none of its own:
the assertion of its topological order stays as a check of the compiler's own work,
where reaching it would be a bug of the front end.
Nothing of the order is repaired or converged on: there is no order, and a program is not built
from modules that cannot be ordered.

### Positive Consequences

- A cycle is reported where it is written, to every host:
  the editor and the command line read one diagnostic family for it, and none of them has to
  build a program to see it.
- The stages after the resolution keep assuming the DAG without checking it:
  MIR orders nothing, the link orders nothing, and neither has a case for a cycle to handle.
- Incrementality follows the names:
  an edit that moves no name moves no `named_modules` value, and a project's cycles are a value
  keyed by the modules of it, so an edit that moves no name in a module is read by one project
  and no further.
- The graph is over resolved names, so it sees what the source means:
  a cycle through a re-export, through a type in a signature, or through a call in a body
  is one kind of mistake with one report.

### Negative Consequences

- The resolution level gains two passes and a value per project, and the rendered diagnostics of
  a module are keyed by the cycles of its project:
  a name that enters or leaves a cycle re-renders the diagnostics of the modules of it.
- The graph reads bodies, through the closure of a check, which is a wider input than the
  resolution's own: a body edit that moves a name the project reads is read by the graph of the
  module, and by the cycles of the project when the name is a name of the graph.
- The message of a report names one module of the cycle --- the one the reported name reaches ---
  and a reader who wants the whole cycle reads the reports of the other modules of it.
- The driver now has a pass that looks for cycles, where [ADR-0008][0008-compiler-driver.md]
  says there is no cycle detection at the driver;
  that clause was about the driver's own pull graph --- which stays stratified ---
  and this record refines it rather than replacing it.

## Pros and Cons of the Options

### Find the cycle at the link stage

- Good, because the order is needed there and nowhere before it,
  and no value of an earlier stage has to change.
- Bad, because a host that resolves names and does not link --- the editor, the LSP ---
  never sees the report, which is what the report is for.
- Bad, because MIR and codegen already expect the DAG:
  a program whose cycle is found at the link is compiled through every stage that assumed it
  could not exist, and the stages are what a bug report is filed against.
- Bad, because the link is the project's, and a cycle is a property of the names:
  the check would be a second version of resolution running in the wrong stage.

### Find the cycle inside the resolution of a module

- Good, because the diagnostic would come out of a pass a host already reads.
- Bad, because a resolution is handed the modules _one module's paths reach_
  ([ADR-0016][0016-inter-module-resolution.md]):
  the modules that reach _it_ --- the other half of every cycle --- are not in its input,
  and giving the pass every module of the project makes it a pass of the project anyway.
- Bad, because the whole graph would be walked once per module of the project, and an edit that
  moves one name would move the diagnostics of every module of it.

### Find the cycle in a pass of the project

Chosen. Described above.

- Good, because a cycle is a property of the whole project, and the pass reads exactly that:
  the named modules of its modules, and nothing else.
- Good, because both values are memoized and keyed by what they read,
  so the cost of an edit follows the modules it moved names in.
- Bad, because the rendered diagnostics of a module now depend on a value of its project,
  which widens the key of a value many edits are read by.

### Report one module of the cycle, or every module of it

- Every module, chosen: the mistake is the source's,
  a reader of any of the files of the cycle is told, and no module is decided to be "the" one
  that closes a cycle that has no beginning.
- One module: good, because one report is less to read and the other modules are named in it.
  Bad, because which module it would be is decided by a rule a reader has to learn,
  and a reader of the other files of the cycle is told nothing.

## Links

- The module system, which the imports of a program may not cycle:
  [0004-module-system.md]
- The pull graph that stays stratified: [0008-compiler-driver.md]
- The contract a pass satisfies, and the diagnostic that is data: [0009-pass-contract.md]
- Module preludes, whose imports are names like any other: [0011-module-prelude.md]
- Resolution against interfaces, and the walk that follows a re-export:
  [0016-inter-module-resolution.md]
- The closure a check reads, which is the closure the graph walks: [0017-resolved-types.md]
- Translation units, whose order is an order of names: [0021-translation-units.md]
- Implementation: [mlkc-resolve], [mlkc-driver]

[0004-module-system.md]: 0004-module-system.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0009-pass-contract.md]: 0009-pass-contract.md
[0011-module-prelude.md]: 0011-module-prelude.md
[0016-inter-module-resolution.md]: 0016-inter-module-resolution.md
[0017-resolved-types.md]: 0017-resolved-types.md
[0021-translation-units.md]: 0021-translation-units.md
[mlkc-resolve]: ../../crates/mlkc-resolve
[mlkc-driver]: ../../crates/mlkc-driver
