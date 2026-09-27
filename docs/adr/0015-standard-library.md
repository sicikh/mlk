# Keep the standard library in the repository, and build it into the compiler

- Status: accepted
- Date: 2026-09-28

## Context and Problem Statement

The names of the language --- `Int`, `Unit`, and the many that will follow ---
are declared by a library written in MLK itself,
and every module of every project is given them without writing the imports
([ADR-0011][0011-module-prelude.md]).
Until now the library existed only in that sentence:
the prelude of the language names `std::prelude::Int`,
and no file anywhere declares `Int`.

Writing it raises three questions that outlive the two files it starts as.

- **Where does the library live?** It is code, so a person reads and reviews it,
  and it is not the code of any project a person compiles: it is the code of the compiler.
- **How does a host tell the driver about it?**
  The driver never opens a file ([ADR-0008][0008-compiler-driver.md]),
  so somebody has to hand the library over,
  and the hosts --- the CLI, the editor, and the language server to come ---
  must not each invent their own answer to what `std` is.
- **What may a person do with it?**
  An editor that shows the library as it shows a buffer invites an edit that cannot be kept:
  the file a person would write is not the file the compiler is reading,
  and the library of the next release would take that edit away.

The question: where does the library live,
how does a host record it in a driver,
and what keeps an editor from treating it as a file of a person's own?

## Decision Drivers

- The library and the prelude of the language are one fact seen from two sides:
  the language names `std::prelude::Int`, the library declares and re-exports it,
  and a change to one that the other does not follow is a compiler that lies about its own names.
- The compiler already knows the library: the prelude it compiles every module with is the
  library's list of names, so what the library is belongs where the compiler is and not in a
  crate that drives the compiler.
- A host owns the file system and the driver never reaches for anything
  ([ADR-0008][0008-compiler-driver.md]): the library is handed over like any other input.
- A browser has no file system, and no directory to read a library from.
- What a person reads and what the compiler compiles must be the same text:
  a version of `Int` read in one place and compiled from another is the failure this decision
  is about.
- The library is the compiler's, not a project's: no host may write in it,
  and an editor is where that has to be visible.

## Considered Options

- **Sources in the repository, built into the compiler** --- the files a person reads are the
  files the compiler holds, taken when it is built.
- **Sources in the repository, read by every host at run time** --- the CLI reads the directory
  of the repository it was invoked from, the browser fetches the files over HTTP.
- **Sources written into a generated Rust file** --- `xtask` reads `library` and writes a file
  the compiler includes, as it already does for the grammar of the editor.
- **The library as strings of a crate** --- the sources are literals, and there are no files
  a person reads and reviews.
- **The library as a crate that drives the driver** --- the sources and what they say of
  a project live in a crate of their own, which registers the library in a driver and depends
  on the driver to do it.
- **Every host records the library itself** --- what `std` is, and which file each of its
  modules is, is written out by the CLI, by the editor, and by every host that follows.

## Decision Outcome

Chosen option: "Sources in the repository, built into the compiler",
because it is the only one of the six that answers all three questions at once:
the files a person reads are the files the compiler holds,
no host needs a file system or a network to have the library,
and what the library is is written once, where the compiler is.

### The library is a directory of the repository, one per project

```
library/
    std/
        core.mlk        module project::core
        prelude.mlk     module project::prelude
```

A project of the library is a directory named after the project,
and a module of it is the file named after the module:
the path of a module is what a person sees in the repository,
and the file it is written in is the module's name with `.mlk` after it.
The names of the two directories come from the language itself:
the prelude of the language names the project `std` and its module `prelude`,
so a repository that spells them differently would be lying about its own library.

### The compiler holds a copy of the sources, taken when it is built

`mlkc-stdlib` embeds the files with `include_str!`, so the library that a compiler compiles
against is the library of the revision the compiler was built from.
This is what makes the drift in the first decision driver a build error rather than a mystery:
a compiler whose prelude names a name the library no longer exports fails its own tests
(`the_prelude_of_the_language_names_what_the_library_exports`),
and a compiler built from a revision always holds that revision's library.

A host that reads the files from a path instead would compile against whatever the working
directory happens to hold, which is how a person ends up debugging a library that is not the
one being compiled.

### The library is the compiler's, and the driver depends on it

`mlkc-stdlib` holds the names, the sources, and the project:
which module the project starts from, the imports the library gives its own modules
([ADR-0011][0011-module-prelude.md]), and the path the compiler keeps each module under.
It drives nothing: it depends on the definitions of the HIR and on the type of a path, and on
nothing else, so what the library is is a value rather than a component with a lifetime.

The dependency points from the driver to the library, and not the other way around,
because the compiler is the side that already knows the library:
the prelude every module is compiled with is the library's list of names,
so a crate that drives the driver would be the compiler's knowledge spelled outside the compiler.

A host calls [`mlkc-driver`]'s `use_std` and is handed the files of the library ---
where each of them lands in the driver, and its source.
The host says _when_ the library goes in, and nothing else: it does not know the project,
the prelude, the paths, or even the names of the modules,
so two hosts cannot disagree about the library of one compiler.

The driver learns nothing about the standard library as such:
it is handed texts and a project, exactly as it is handed the files of a person's project,
and it has no state that says "this module is the library".
The module graph it already holds is where the fact lives
(`ProjectData` and `set_module_project`, [ADR-0008][0008-compiler-driver.md]).

### A host asks for the library; it does not push it

The browser host asks the driver for the library ([`mlkc-wasm`]'s `useStd`) and is handed the
files it is made of, because there is nothing for a browser to read and no second copy to keep
in step.
The editor takes those files as it takes any other buffer:
it shows them, it asks the compiler about them, and it is the one that decides what a person
may do with them.

A driver whose host never asks holds no library, which is deliberate:
the tests of the driver want a driver that is only what the test pushed,
and a host that compiles against another library records its own project.
The library goes in after the buffers a host pushed itself,
so what a host did first keeps the file ids it had.

### The driver takes what it is pushed; immutability is the host's

A read-only buffer is not a property of the compiler: the driver has no policy about who may
push what, and a host with a reason to push a library --- a test, a tool that answers
"what if `Int` were different" --- is not forbidden from doing so.
What the editor owes a person is that _it_ does not write the library, and does not offer to:
a file of the library is opened with a state that takes no text and a view that is not editable,
its rows offer no way to drop it, and the directory it sits in is not a place to make a buffer.
The editor learns which files those are from the driver it asked for the library,
so it never has to know the path of a library by itself.

### Positive Consequences

- Two files in the repository are the library: the compiler, the editor, and a person reading
  the repository all have the same text.
- A host that begins to exist later --- a language server, a test harness --- asks the driver
  for the library and cannot spell its project differently from the others: it does not know
  enough about the library to have an opinion.
- The compiler is the side that carries the library, which is where the knowledge already was:
  the prelude the driver compiles every module with is the library's list of names.
- The driver keeps its shape: it gained no state about the library, only a method that pushes
  text and records a project, which is what it does for every other input.

### Negative Consequences

- Editing a library file needs a rebuild of the wasm module before the editor shows it:
  a normal development loop (`pnpm dev` and `cargo build`) does it, but a copy of the library
  that is already in a running page stays as it was built.
- The library is compiled in whatever form it was in at build time,
  and a compiler cannot be pointed at another one: overriding `std` waits for a sysroot
  and a loader, which the pipeline does not have.
- The sources of the library are in the binary of every host that links the driver --- including
  a test that never asks for it. The library is small today; a standard library that grows
  will have to be measured, and maybe fetched rather than embedded.
- A crate of its own for two files is machinery ahead of its contents;
  it is the crate's _shape_ --- the sources, the project, and the prelude of the library in
  one place, below the compiler --- that the decision is about.

## Pros and Cons of the Options

### Sources in the repository, read by every host at run time

- Good, because a change to a library file is seen by the next run, with nothing to rebuild.
- Bad, because the CLI would read the library of whatever directory it was invoked from,
  which is a different library from the one the compiler was built with,
  and a different one again for a build from another checkout.
- Bad, because the browser cannot read the repository: it would fetch the files from the site,
  which puts a copy of the library in the build of the editor --- a copy that can be stale,
  and one more thing that has to be kept in step.
- Bad, because a host that has no file system has nothing to read at all,
  and the "read at run time" answer becomes two answers.

### Sources written into a generated Rust file

- Good, because the sources stay files of the repository, and the generated file is checked by
  the same "nothing that is generated is out of date" step the grammar of the editor already is.
- Bad, because it is the same thing as embedding them with one more step in the middle:
  `include_str!` is a generator that has no configuration, no output to keep in the tree,
  and no way to go stale.

### The library as strings of a crate

- Good, because the compiler is the only place the library exists, and nothing can drift.
- Bad, because a library is code a person reads, reviews, and diffs: a string literal is not
  where a language's standard library is written, and no editor could show it as source.

### Every host records the library itself

- Good, because the driver gains nothing, and the hosts stay equal: each of them pushes text
  and records projects, which is the whole of the driver contract.
- Bad, because "what `std` is" is then written once per host: four names, two module paths, and
  a prelude, in the CLI, in the editor, and in every host that follows, with nothing to keep
  them in agreement but review.

### The library as a crate that drives the driver

The shape this decision first had: the sources live in a crate that registers the library in a
[`mlkc-driver`], and the crate depends on the driver to do it.

- Good, because a host calls one function and knows nothing but the paths it wants the library
  under, which the crate asks it for.
- Bad, because the dependency points the wrong way: the compiler is the side that already knows
  the library, so the crate that drives the driver is the compiler's own knowledge spelled
  outside the compiler, and a second crate that wanted that knowledge would have to take the
  driver with it.
- Bad, because the paths become the host's to choose, which is a thing no host has wanted:
  a host that spells the library its own way is a host that disagrees with its compiler about
  where the library is.

## Links

- The prelude, and where the library's names enter a module: [0011-module-prelude.md]
- Inputs by push, hosts, and the module graph: [0008-compiler-driver.md]
- The language the library is written in: [0002-lossless-syntax-tree.md],
  [0004-module-system.md]
- Implementation: [mlkc-stdlib], [mlkc-driver], [mlkc-wasm], [mlkc-cli], the editor

[0002-lossless-syntax-tree.md]: 0002-lossless-syntax-tree.md
[0004-module-system.md]: 0004-module-system.md
[0008-compiler-driver.md]: 0008-compiler-driver.md
[0011-module-prelude.md]: 0011-module-prelude.md
[mlkc-stdlib]: ../../crates/mlkc-stdlib
[mlkc-driver]: ../../crates/mlkc-driver
[mlkc-wasm]: ../../crates/mlkc-wasm
[mlkc-cli]: ../../crates/mlkc-cli
