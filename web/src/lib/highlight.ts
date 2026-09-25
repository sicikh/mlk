/**
 * The language, as the editor knows it: a grammar of MLK and the colours of what it finds.
 *
 * Painting is the client's business, the way it is in every editor. A language server
 * hands out semantic tokens — the names that are resolved, the types, the meanings —
 * and leaves the syntax to whoever is holding the text; of those two, this file is the
 * second, and the first is not built yet: the compiler does not resolve a name to
 * what it means as something an editor can ask for.
 *
 * So the colours come from a parse of the editor's own, next to the compiler's — and a
 * much smaller one, because all it has to do is say what a piece of text *is*: a tree
 * only decides colours here, never whether the code is right. That stays the compiler's:
 * what it complains about is marked by the diagnostics, not by the grammar.
 * The grammar is allowed to be behind the language it follows: what it does not know is
 * not an error, it is text with the colour of text.
 */
import {
    HighlightStyle,
    LanguageSupport,
    LRLanguage,
    syntaxHighlighting,
} from "@codemirror/language";
import { styleTags, tags } from "@lezer/highlight";

import { parser } from "./grammar/mlk";

/**
 * What each part of the syntax is called.
 *
 * A name is not listed: a page of code is mostly names, and painting them all makes the
 * rest harder to read. Only the names the grammar can tell apart from the others are
 * painted — a path written where a type belongs is a type name — and everything else keeps
 * the colour of text. A path written where a value belongs names whatever resolution says
 * it names, which a grammar does not know, so nothing of it is painted.
 */
const syntax = styleTags({
    // The keywords, which the grammar reads out of names: see `kw<...>` in the grammar.
    "fun in let module type _": tags.keyword,
    Comment: tags.comment,
    IntLiteral: tags.number,
    StringLiteral: tags.string,
    "Type/Path/PathSegment/Name": tags.typeName,
});

/** The grammar, dressed as the language of a CodeMirror view. */
export const mlkLanguage = LRLanguage.define({
    name: "mlk",
    parser: parser.configure({ props: [syntax] }),
    languageData: {
        commentTokens: { line: "//", block: { open: "/*", close: "*/" } },
    },
});

/** What a painted part of the syntax looks like: the colours of the page, worn by code. */
const style = HighlightStyle.define([
    { tag: tags.keyword, color: "var(--accent)" },
    { tag: tags.comment, color: "var(--muted)", fontStyle: "italic" },
    { tag: tags.number, color: "var(--warning)" },
    { tag: tags.string, color: "var(--ok)" },
    { tag: tags.typeName, color: "var(--type)" },
]);

/** The language of the editor, ready to be handed to a view. */
export function mlkLanguageSupport(): LanguageSupport {
    return new LanguageSupport(mlkLanguage, [syntaxHighlighting(style)]);
}
