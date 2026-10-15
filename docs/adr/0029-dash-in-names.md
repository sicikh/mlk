# Read a `-` as part of a name only between name characters

- Status: accepted
- Date: 2026-10-16

## Context and Problem Statement

Names in MLK are written in kebab case:
`println-int`, `start-app`, `no-prelude`.
The hyphen is a character of the language's alphabet,
and at the same time it is the minus operator,
and, before a `>`, the first character of the arrow a step of a chain is written with ([ADR-0012][0012-split-the-dot-operator.md]).
The lexer decides, at every `-`, which of the three is written,
and it decides from the text alone:
a token is what it is wherever it stands,
and none of the three needs a declaration or a surrounding construct to be read
([ADR-0005][0005-compiler-pipeline.md]).

The decision is visible while a program is being written.
A host rebuilds the tree as the text changes and says what the tree holds,
so the tree of a half-written step is what the reader is told about:

- `x` holds the name `x`, and the host resolves it;
- `x-` holds the name `x` and the operator `-`,
  and the mistake is the operand the operator is missing;
- `x->` holds the name `x` and the arrow,
  and what is missing is the callee, which the host completes
  from the names the module's scope holds ([ADR-0011][0011-module-prelude.md]);
- `x->f` holds the name, the arrow, and the callee,
  and the call it wants is the parenthesis.

If a `-` could end a name, the tree of `x-` would hold the name `x-` instead,
which no module declares:
a host would report a name nobody wrote and could offer nothing,
and the reader would be told they are wrong at the moment the editor could help.
The rule that decides the three uses is therefore what makes
the arrow spellable while it is typed, not a detail of the lexer.

The question: where may a `-` be part of a name?

## Decision Drivers

- Names are kebab case, and a `-` between the parts of a name is part of the language's style.
- Arithmetic on those names stays writable: `x - y` subtracts two names that hold dashes.
- The rule is a lexer rule:
  it is read from the characters of the text around the `-`,
  it is regular like every token of the language ([mlkc-parser], [grammar]),
  and no parser state and no declaration takes part in it.
- A prefix of a construct being typed does not become a name no one wrote,
  and never becomes an operand where a reader is writing a step.
- One rule, without a case for a spelling the language has no construct for.
- The arrow `->` is one token, and a name cannot swallow it ([ADR-0012][0012-split-the-dot-operator.md]).

## Considered Options

- **A `-` may stand anywhere in a name** --- leading, trailing, and doubled.
- **A `-` joins a name only before a character that may begin one** --- letters and `_`, not digits.
- **A `-` is a name character only between two name characters** --- the rule read today.
- **`--` is a lexical error** --- the chosen rule, with two dashes in a row refused.

## Decision Outcome

Chosen option: "A `-` is a name character only between two name characters".

### The rule

A name begins with a letter or `_`,
and its characters are letters, digits, `_`, and --- since the records beside this one ---
a prime wherever a name character may stand ([ADR-0030][0030-primes-in-names.md]).
The letters are the letters of any alphabet ([ADR-0031][0031-unicode-names.md]),
not the letters of ASCII:
the classes of a name are `XID_Start` and `XID_Continue`.
A `-` is one of its characters only when it stands directly between two of them:

```
name := (letter | '_') (letter | digit | '_' | '\'')* ('-' (letter | digit | '_' | '\'')+)*
```

A name therefore never begins and never ends with a `-`.
Every other `-` is the operator `-`,
and where a `>` follows it, the lexer reads the longest token that starts there,
which is the arrow `->`.

A run that begins with a digit is a number, not a name,
so a `-` after a number is always the operator:
`1-2` is a subtraction, whatever the same characters would mean after a letter.

The rule is what the lexer of the compiler reads ([mlkc-parser]);
this record makes it a decision,
because the step of a chain ([ADR-0012][0012-split-the-dot-operator.md]) depends on it
and because it is the kind of rule that is relaxed by accident.
The alphabet of a name and the prime it may hold are decisions of their own
([ADR-0030][0030-primes-in-names.md], [ADR-0031][0031-unicode-names.md]);
what this record decides is the `-`.

### What the rule reads

| Written    | Read as                | Because                                                             |
| ---------- | ---------------------- | ------------------------------------------------------------------- |
| `x-y`      | one name `x-y`         | the `-` stands between two name characters                          |
| `x - y`    | `x`, `-`, `y`          | a space is not a name character, so the `-` stands outside a name   |
| `x- y`     | `x`, `-`, `y`          | the character after the `-` is a space                              |
| `x -y`     | `x`, `-`, `y`          | the character before the `-` is a space, and the sign is unary      |
| `x-1`      | one name `x-1`         | a digit is a name character                                         |
| `x- 1`     | `x`, `-`, `1`          | the character after the `-` is a space                              |
| `x-`       | `x`, `-`               | a name does not end with a `-`, so the `-` is the operator          |
| `x->`      | `x`, `->`              | a name does not end with a `-`, and `->` is the longest token there |
| `-foo`     | `-`, `foo`             | a name does not begin with a `-`, and the sign is unary             |
| `1-2`      | `1`, `-`, `2`          | a name does not begin with a digit, so no name is being read        |
| `foo--bar` | `foo`, `-`, `-`, `bar` | the first `-` is followed by a `-`; the second follows an operator  |

### `--` is two operators, not a token

`foo--bar` is `foo - (-bar)`, the tight spelling of `foo - -bar`:
the language reads a sign written twice already (`- -1`), and these are two signs of it.
No `--` token is read, and two dashes in a row are not an error.

A lexical error was considered and refused.
A token is what a construct of the language is written with,
and the language has no construct a `--` would name:
a rule that only forbids would be a rule every lexer and every reader has to learn,
and it would take away a spelling that already parses into the only value it could mean.
A reader who meant `foo--bar` as one name gets the parser's report at the first `-`,
which is where the name ended;
no rule about dashes has to name the mistake.

### What a host says while a step is typed

The table above is what the tree holds; this is what a host does with it,
and it is the reason the rule is as strict as it is:

- `x-` is a name and a missing operand:
  the name resolves, and the diagnostic points at the operator,
  so a reader who is mid-way through `x - 1` or `x->f(1)` is helped in both;
- `x->` is a name and an arrow:
  the callee is what stands next, and the candidates are the names of the module's scope
  ([ADR-0011][0011-module-prelude.md], [ADR-0012][0012-split-the-dot-operator.md]);
- `x->f` is a name, an arrow, and a callee:
  the arrow has what it calls, and what the call wants is an argument list.

With a `-` that a name could end on, none of the three would be a step:
`x-` would be a name of its own, reported as one,
and the host would have nothing to offer at the exact keystroke where the arrow begins.

### Positive Consequences

- Names keep their dashes: `println-int`, `start-app`, and `utf-8` are names.
- Arithmetic keeps its numbers: `x-1` is a name, and `x -1`, `x- 1`, and `x - 1` are subtractions.
- A prefix of a step never becomes a name:
  `x-` and `x->` hold what was written, and a host can help while the step is typed
  ([ADR-0012][0012-split-the-dot-operator.md]).
- The rule is one sentence about the characters around one character,
  and it is the same sentence in the compiler's lexer and in the editor's grammar.

### Negative Consequences

- The space between a name and a minus is part of the program:
  `x-y` is a name, and a reader of other languages writes `x-y` for a subtraction
  until they learn to write `x - y`.
- `x-1` is a name, so the subtraction of `1` from `x` written tightly is a name that does not exist;
  the subtraction wants a space somewhere.
- The rule lives in more than one lexer --- the compiler's and the editor's ---
  and the two have to stay in step;
  the tests of both have to watch the examples above.

## Pros and Cons of the Options

### A `-` may stand anywhere in a name

- Good, because any name a project wants can be written: `-foo`, `x-`, `x--y`.
- Bad, because a name could begin with a `-`, and then `-foo` is a name and a negation at once:
  no lexer can read it, because nothing around the `-` says which one is written.
- Bad, because the prefix of a step becomes a name:
  `x-` would be the name `x-`, reported as an unknown name
  instead of read as the name `x` and the operator a step begins with.

### A `-` joins a name only before a character that may begin one

Letters and `_` continue a name across a `-`; digits do not.

- Good, because `x-1` is a subtraction, and arithmetic on a name and a number is written tight.
- Bad, because a name cannot hold a number after a dash:
  `sha-256` would have to be `sha256`,
  and the alphabet of a name would be cut by the wants of arithmetic.
- Bad, because the same digit would be a name character in `x1` and arithmetic in `x-1`,
  which is a case a reader has to hold apart for one character class.

### A `-` is a name character only between two name characters

Chosen. Described above.

- Good, because names keep their dashes and arithmetic keeps its numbers.
- Good, because the prefix of a step is never a name,
  which is what lets a host read `x->f(args)` as it is typed ([ADR-0012][0012-split-the-dot-operator.md]).
- Good, because the rule is read in one pass, without context,
  and the same way in every lexer of the project.
- Bad, because `x-y` is a name and the subtraction wants a space,
  so whitespace decides meaning at the one place the language cannot avoid it.

### `--` is a lexical error

- Good, because two dashes in a row are almost always a slip,
  and the error would point at them.
- Bad, because it would forbid a spelling the language already reads:
  `foo--bar` is `foo - (-bar)`, and `- -bar` and `-(-bar)` write the same value.
- Bad, because a token for a spelling the language has no construct for
  is machinery in every lexer ([mlkc-parser], [grammar], and the language server's).

## Links

- The arrow a step is written with, and the typing it has to survive: [0012-split-the-dot-operator.md]
- The prime a name may hold: [0030-primes-in-names.md]
- The alphabet of a name: [0031-unicode-names.md]
- Lexing as a stage of the pipeline: [0005-compiler-pipeline.md]
- The names a host completes: [0011-module-prelude.md]
- Implementation: the lexer of [mlkc-parser], and the editor's [grammar]

[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0011-module-prelude.md]: 0011-module-prelude.md
[0012-split-the-dot-operator.md]: 0012-split-the-dot-operator.md
[0030-primes-in-names.md]: 0030-primes-in-names.md
[0031-unicode-names.md]: 0031-unicode-names.md
[mlkc-parser]: ../../crates/mlkc-parser
[grammar]: ../../web/src/lib/grammar/mlk.grammar
