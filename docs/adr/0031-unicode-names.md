# Let a name hold the letters of any alphabet

- Status: accepted
- Date: 2026-10-16

## Context and Problem Statement

The characters of a name are ASCII today:
the letters `a`--`z` and `A`--`Z`, the digits `0`--`9`, `_`,
and the `'` and `-` of the records beside this one ([ADR-0029][0029-dash-in-names.md], [ADR-0030][0030-primes-in-names.md]).
A reader who thinks in Greek, Cyrillic, or Devanagari writes their names in that alphabet
and transliterates them for the compiler,
or names the things of a program in English while thinking in another language.
A language whose names are words should not take its alphabet from its first implementation.

The question: which characters may a name hold?

## Decision Drivers

- The alphabet of the reader:
  a name is what its reader calls the thing,
  and no reader should have to write it in the alphabet of somebody else.
- `XID_Start` and `XID_Continue` (UAX #31) are the properties the languages around MLK use
  for identifiers --- Rust, C#, and the `ID_Start`/`ID_Continue` of JavaScript;
  they hold the letters, the digits, and the combining marks of every script,
  and they are closed under NFKC.
- The marks are not decoration:
  a Devanagari vowel sign, an Arabic vowel mark, and a combining accent are parts of a word,
  so a rule of letters and digits alone cannot spell names in the scripts that write them.
- The rule stays regular and is read without context ([ADR-0029][0029-dash-in-names.md]):
  the classes of its characters change, the shape of the rule does not.
- What is not a letter is not a name character:
  an arrow, an em dash, and the typographic prime stay outside,
  so a name in the source is what it looks like, and a typo stays an error.
- The lexer of the compiler and the grammar of the editor state the same rule,
  as far as each of them can ([ADR-0029][0029-dash-in-names.md]).

## Considered Options

- **ASCII only** --- the decision that stands.
- **The letters and the digits of every script, without marks** --- the categories `L` and `Nd`.
- **`XID_Start` and `XID_Continue`** --- the identifier characters of UAX #31.
- **Every character that is not whitespace or a delimiter** --- the permissive rule an editor
  sometimes states.

## Decision Outcome

Chosen option: "`XID_Start` and `XID_Continue`",
because they are the standard property set for the identifiers of a language,
because they are closed under NFKC,
and because they are what a reader who has written an identifier in any other language expects.

### The rule

A name begins with a character of `XID_Start` or `_`;
its characters are the characters of `XID_Continue`, a `'` wherever a name character may stand
([ADR-0030][0030-primes-in-names.md]), and a `-` between name characters ([ADR-0029][0029-dash-in-names.md]):

```
name := (xid_start | '_') rest* ('-' rest+)*
rest := xid_continue | "'"
```

`XID_Continue` holds the digits and the marks of every script,
so a name is a name whatever way its numbers, its vowels, and its accents are written:

| Written      | Read as            | Because                                                                             |
| ------------ | ------------------ | ----------------------------------------------------------------------------------- |
| `café`       | one name           | `é` is a letter of an alphabet                                                      |
| `αβ`         | one name           | Greek letters are `XID_Start` and `XID_Continue`                                    |
| `кафе`       | one name           | so are Cyrillic ones                                                                |
| `गिनती`      | one name           | `ि` and `ी` are `XID_Continue`: a vowel sign continues a name                       |
| `각-글`      | one name           | the dash stands between name characters ([ADR-0029][0029-dash-in-names.md])         |
| `e` + U+0301 | one name           | a combining accent continues a name                                                 |
| `x→y`        | `x`, an error, `y` | an arrow is not a letter                                                            |
| `f′`         | `f`, then an error | the typographic prime is not a name character ([ADR-0030][0030-primes-in-names.md]) |

### What a name compares by is not decided here

This record decides which characters a name may hold, and nothing about how two names compare.
What stands is the plain reading: a name is the sequence of code points the module wrote,
and two spellings of one letter are two names.
The cost is real and is named here:
`café` with a precomposed `é` and `café` with `e` and a combining accent are two names
that a reader reads as one,
and a diagnostic about one of them names the other to the eye.
UAX #31 recommends a normalization (NFC or NFKC) for exactly this reason,
and whether the compiler normalizes a name when it reads it --- and which form ---
is a decision of its own,
with consequences for the identity of a declaration, for the text a diagnostic shows,
and for the tool that renames;
this record leaves it open rather than guessing it.

### The grammar of the editor holds a superset

The grammar of the editor ([`mlk.grammar`]) cannot list the characters of `XID_Start`:
they are some thousands of ranges, and the grammar is written by hand.
It holds a superset instead: any non-ASCII character is a character of a name there.
The cost is a colour, and never a verdict:
a character that is not a letter is part of a name in the editor and not in the compiler,
and the editor never judges what the language is
([`highlight.ts`] states the principle: the grammar paints, and the compiler reports).

### What a build names a module's file

A name reaches the file system:
a translation unit is written under the file a canonical name becomes ([`Manifest::file_of`], [ADR-0021][0021-translation-units.md]),
which keeps what a path holds and turns what it may not hold into a dash.
The letters of a name are kept now, whatever alphabet they are written in,
so the file of `project::кафе` is `кафе.wasm`
and not `----.wasm`, which two names would have shared.

### Positive Consequences

- A name is written in the alphabet of its reader:
  `αβ`, `кафе`, `गिनती`, and `각-글` are names like `println-int` is one.
- The marks make the rule complete for the scripts that need them:
  a name is not cut at the vowel sign of a Devanagari word or the accent of an Arabic one.
- The shape of the rule is the one it had:
  a name is still one regular rule read without context,
  and every rule built on it --- the dash, the prime, the grammar of the editor ---
  reads the same characters.
- No stage after the lexer takes a name apart:
  a name is a string from the token to the build, and nothing validates its characters again.

### Negative Consequences

- Normalization is open (above):
  until it is decided, two spellings of one letter are two names,
  and a diagnostic about one of them points at a name the reader reads as the other.
- The lexer carries two Unicode classes into its generated DFA,
  so the binary and the time it takes to compile grow a little.
- The grammar of the editor and the lexer of the compiler differ at the characters a name may
  not hold, so what the editor paints as one name is two tokens for the compiler.
- A name may be written in an alphabet a reader of the source does not read,
  and nothing in the language says which language a name is in;
  a project is free to restrict its names, and the compiler does not.

## Pros and Cons of the Options

### ASCII only

- Good, because the lexer, the editor, and every tool that reads a name hold one small alphabet,
  and no question of normalization arises.
- Bad, because it makes the alphabet of the implementation the alphabet of every name,
  and asks every reader who does not think in Latin letters to transliterate their own words.
- Bad, because the language claims that a name is what its reader calls a thing,
  and this rule makes that true for some readers only.

### The letters and the digits of every script, without marks

- Good, because a name of a script that needs no marks is spelled the way its reader spells it.
- Bad, because it cannot spell the names of the scripts that need marks:
  `गिनती` would be `ग`, an error at the vowel sign, and `ती`,
  which is a rule the reader of that script cannot follow at all.
- Bad, because the categories would have to be read one by one (`L` and `Nd`),
  where `XID_Start` and `XID_Continue` are one answer with a body behind it (UAX #31).

### `XID_Start` and `XID_Continue`

Chosen. Described above.

- Good, because they are the standard set for identifiers,
  and a reader who has written Rust, C#, or Python has already written identifiers with them.
- Good, because marks and digits come with them, so the scripts that need them are spelled
  in full.
- Good, because the set is closed under NFKC,
  which is what makes a later decision about normalization possible at all.
- Bad, because the alphabet is a table of Unicode and is not read off the source of the lexer
  like the ASCII classes were.
- Bad, because a reader cannot tell by looking whether an unusual character is a letter;
  the compiler says so with a diagnostic, which is what a diagnostic is for.

### Every character that is not whitespace or a delimiter

- Good, because a name is anything a reader writes, and no table has to be kept.
- Bad, because then `→`, `—`, and `«` are characters of names,
  and what the language reads stops being what it looks like:
  a typo stops being an error, which is the price every permissive rule pays.

## What this record does not decide

- Normalization: whether a name is normalized when it is read, which form, and what it means
  for the identity of a declaration and the text of a diagnostic (above).
- Whether a keyword may ever be written in another alphabet:
  no, and this record does not touch the words of the language.
- Whether a name is one word in one language:
  a project may hold its names to a convention, and the compiler holds them to this record only.

## Links

- The rule of the name, and the dash: [0029-dash-in-names.md]
- The prime a name may hold: [0030-primes-in-names.md]
- Lexing as a stage of the pipeline: [0005-compiler-pipeline.md]
- Translation units, whose files a name becomes: [0021-translation-units.md]
- UAX #31, Unicode Identifier and Pattern Syntax: <https://unicode.org/reports/tr31/>
- Implementation: the lexer of [mlkc-parser]; the editor's [`mlk.grammar`] and [`highlight.ts`];
  the file a name becomes in [`Manifest::file_of`]

[0005-compiler-pipeline.md]: 0005-compiler-pipeline.md
[0021-translation-units.md]: 0021-translation-units.md
[0029-dash-in-names.md]: 0029-dash-in-names.md
[0030-primes-in-names.md]: 0030-primes-in-names.md
[mlkc-parser]: ../../crates/mlkc-parser
[`mlk.grammar`]: ../../web/src/lib/grammar/mlk.grammar
[`highlight.ts`]: ../../web/src/lib/highlight.ts
[`Manifest::file_of`]: ../../crates/mlkc-driver/src/manifest.rs
