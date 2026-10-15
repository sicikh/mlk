# Let a name hold `'`

- Status: accepted
- Date: 2026-10-16

## Context and Problem Statement

Names of MLK are written in kebab case ([ADR-0029][0029-dash-in-names.md]):
`println-int`, `start-app`.
What the language cannot write is the mark the languages around it use
when a name needs a next version of itself:
Haskell, OCaml, and SML write `f'` for a function derived from `f`,
`f''` for the one after that,
and a reader of mathematics reads `x'` as "the next `x`" without being told.
MLK has only the long way: a word for every step (`validate-again`, `start-next`)
or a new name for each version.

The question: where may a `'` stand in a name?

## Decision Drivers

- The mark is a suffix: `f'` reads as the name `f` with one prime,
  and a reader of the languages that write primes already reads it that way.
- A name stays one rule: a prime joins the characters a name already holds,
  and the dash rule ([ADR-0029][0029-dash-in-names.md]) keeps reading the characters around a `-`.
- A name does not begin with a prime,
  so the character is free where a token starts:
  a quoted form of a character, if the language ever writes one, is not argued with here.
- A lexer reads a token without context:
  the rule is a regular language like every other part of a name,
  and the lexer of the compiler and the grammar of the editor state it alike.
- A prime does not disturb typing:
  `x->f()` is a name, an arrow, and a call ([ADR-0029][0029-dash-in-names.md]),
  and a prime is neither a `-` nor a `>`.

## Considered Options

- **No prime** --- the decision that stands: a derived name gets a word or a name of its own.
- **A prime only at the end of a name** --- `f'` and `f''`, but not `f'g`.
- **A prime wherever a character of a name may stand after the first one** --- `f'`, `f''`, `f'g`.
- **A prime in any position, the first one included** --- `'f`.

## Decision Outcome

Chosen option: "A prime wherever a character of a name may stand after the first one",
because it is the rule of the languages that write primes,
and because it says one thing about the character:
a prime is a character of a name wherever a digit is one.

### The rule

[ADR-0029][0029-dash-in-names.md] holds the rule of the whole name;
this record adds a character to it.
The prime is a character of a name after its first one:

| Written   | Read as            | Because                                                         |
| --------- | ------------------ | --------------------------------------------------------------- |
| `f'`      | one name `f'`      | the prime is a character of the name after its first one        |
| `f''`     | one name `f''`     | as many primes as are written                                   |
| `f'g`     | one name `f'g`     | the prime stands where a digit may stand                        |
| `'f`      | `'`, `f`           | a name does not begin with a prime, and `'` is an error token   |
| `f'-x`    | one name `f'-x`    | the dash stands between name characters, and the prime is one   |
| `x' - y'` | `x'`, `-`, `y'`    | a space keeps the dash out of the name                          |
| `f′`      | `f`, then an error | the typographic prime (U+2032) is not the prime of the language |

The last row is what the record decides about the mark itself:
the prime of the language is one character, the ASCII `'`.
The marks that look like it are not it:
`′` (U+2032) and `’` (U+2019) are not characters of a name at all,
and `ʼ` (U+02BC), which is a modifier letter,
is a character of a name of its own ([ADR-0031][0031-unicode-names.md]),
never the prime of a name:
`fʼ` and `f'` are two names, and no stage reads one as the other.

### What a prime means

Nothing: a prime is a character of a name like a digit.
`f'` and `f` are two names, exactly as `x1` and `x` are two names,
and nothing relates them:
the language does not pair them, does not generate one from the other,
and no stage may report that the one is missing beside the other.
The reading of the mark --- "the next one" --- is a convention of the readers of the language,
the way it is in the languages the prime is borrowed from.

### Positive Consequences

- A derived name is written in one character: `f'` and `f''` where a word would stand.
- The rule of the dash does not change:
  a prime is a name character like a digit,
  so a `-` joins around it as it joins around anything else.
- A name that ends in a prime is a name like any other:
  a declaration, a reference, a path segment, and a parameter are written with it
  without a rule of their own.
- The prime of the language is unambiguous:
  the ASCII `'`, and the marks that look like it are not name characters that could be mistaken
  for it.

### Negative Consequences

- Two names a reader relates, `f` and `f'`, are unrelated to every stage:
  nothing checks that `f'` stands beside an `f`, and a rename does not follow the primes.
- The names differ by one character,
  so a typo between `f'` and `f` is an unknown name like any other typo,
  and a reader has to look at the primes to tell two functions apart.
- `'` joins the alphabet of names, so a language feature that would want the character
  --- a quoted character, say --- starts from a position where a name already holds it inside
  and at its end, and only the start of a token is free.

## Pros and Cons of the Options

### No prime

- Good, because a name stays a word, and a reader of any language reads it without a mark to
  learn.
- Bad, because every version of a name costs a word or a rename,
  and nothing in the text says which name came first.

### A prime only at the end of a name

- Good, because `f'` and `f''` are names, which is what a reader of the mark writes most.
- Bad, because a prime in the middle ends the name:
  `f'g` is the name `f'` and the name `g`,
  which is a mistake a writer does not see coming, and the rule of the character grows a case
  (a prime is not a character of a name when something follows it) that a reader has to hold.

### A prime wherever a character of a name may stand after the first one

Chosen. Described above.

- Good, because the rule is one sentence about the character,
  and it composes with the rule of the dash without a case of its own.
- Good, because it is the rule of the languages that write primes, where a reader has met it.
- Bad, because a name of a name is easy to mistype, and only the primes tell them apart.

### A prime in any position, the first one included

- Good, because the character would be a name character with no exception at all.
- Bad, because `'f` would be a name,
  and a reader could not tell the mark of a name from the mark of a quoted form:
  whatever the language quotes with `'` later, the name rule argues with it,
  and no lexer can read `'f'` one way.

## Links

- The rule of the name the prime joins: [0029-dash-in-names.md]
- The alphabet of the name the prime joins: [0031-unicode-names.md]
- Lexing as a stage of the pipeline: [0005-compiler-pipeline.md]
- Implementation: the lexer of [mlkc-parser], and the editor's [grammar]

[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0029-dash-in-names.md]: 0029-dash-in-names.md
[0031-unicode-names.md]: 0031-unicode-names.md
[mlkc-parser]: ../../crates/mlkc-parser
[grammar]: ../../web/src/lib/grammar/mlk.grammar
