/**
 * The WebAssembly text format, as the viewer of the module knows it: what it paints, and where
 * it folds.
 *
 * The format is an S-expression of words, and a small one: a tokenizer is enough to paint it,
 * and the parentheses themselves say where a form begins and where it ends, so folding needs no
 * tree. That is why there is no grammar here, unlike `highlight.ts`: a grammar would be a build
 * step (`build:grammar`) for a language the compiler writes and no person edits, and the text
 * this reads is the text the compiler already wrote.
 *
 * The colours are the ones the editor paints MLK in ([`codeStyle`]): the module is read beside
 * the buffer, and a keyword is the accent in both.
 */
import {
    LanguageSupport,
    StreamLanguage,
    foldService,
    syntaxHighlighting,
    type StringStream,
} from "@codemirror/language";
import { tags } from "@lezer/highlight";
import type { Text } from "@codemirror/state";

import { codeStyle } from "./highlight";

/** The words a type of the format is written with: a word that names one is painted as a type. */
const TYPES = new Set([
    "i8",
    "i16",
    "i31",
    "i32",
    "i64",
    "f32",
    "f64",
    "v128",
    "ref",
    "funcref",
    "externref",
    "anyref",
    "eqref",
    "i31ref",
    "structref",
    "arrayref",
    "nullref",
    "nullfuncref",
    "nullexternref",
    "exnref",
    "contref",
]);

/** A number of the format: an integer, a float, or one of the two infinities. */
const NUMBER =
    /^[+-]?(0x[0-9a-fA-F](_?[0-9a-fA-F])*|inf|nan(:0x[0-9a-fA-F](_?[0-9a-fA-F])*)?|\d(_?\d)*(\.[\d_]*)?([eE][+-]?\d(_?\d)*)?)/;

/** A word of the format: an instruction (`i32.add`), a type constructor, or a keyword. */
const WORD = /^[@A-Za-z_][\w.-]*/;

/** What the tokenizer keeps between the lines: how many block comments are open. */
interface State {
    /** The block comments that are open: the format nests them. */
    comment: number;
}

/**
 * The format as a stream of tokens, which is all a viewer of it needs.
 *
 * A word that is not a value type is a keyword: the format has no names of its own besides the
 * ones a module gives (`$app::main`), so everything else a word can be is an instruction or one
 * of the words a form is written with.
 */
const wat = StreamLanguage.define<State>({
    name: "wat",
    startState: () => ({ comment: 0 }),
    token(stream, state) {
        if (state.comment > 0) return blockComment(stream, state);

        if (stream.eatSpace()) return null;

        if (stream.match(";;")) {
            stream.skipToEnd();

            return "comment";
        }

        // `(;)` is a comment of nothing: the `;` of the opening is the one of the closing.
        if (stream.match("(;")) {
            if (stream.eat(")")) return "comment";

            state.comment = 1;

            return blockComment(stream, state);
        }

        if (stream.match('"')) return string(stream);

        // An identifier: what a module names a thing of its own by, `$app::main` and all.
        if (stream.match(/^\$[^\s()";]+/)) return "variableName";

        if (stream.match(NUMBER)) return "number";

        const word = stream.match(WORD);

        if (typeof word === "object" && word !== null) {
            return TYPES.has(word[0]) ? "typeName" : "keyword";
        }

        stream.next();

        return null;
    },
    languageData: {
        commentTokens: { line: ";;", block: { open: "(;", close: ";)" } },
    },
    tokenTable: {
        keyword: tags.keyword,
        typeName: tags.typeName,
        variableName: tags.variableName,
        number: tags.number,
        string: tags.string,
        comment: tags.comment,
    },
});

/** Reads a string, which the format quotes and escapes like every other language. */
function string(stream: StringStream): string {
    while (!stream.eol()) {
        const next = stream.next();

        if (next === "\\") stream.next();
        else if (next === '"') break;
    }

    return "string";
}

/**
 * Reads a block comment to the `;)` that closes it, or to the end of the line.
 *
 * A block comment nests and spans lines, so what it leaves open is in the state the next line
 * begins with.
 */
function blockComment(stream: StringStream, state: State): string {
    while (!stream.eol()) {
        if (stream.match(";)")) {
            state.comment -= 1;

            if (state.comment === 0) break;
        } else if (stream.match("(;")) {
            state.comment += 1;
        } else {
            stream.next();
        }
    }

    return "comment";
}

/**
 * Where a form folds: from the end of the line it opens on to the parenthesis that closes it,
 * which is what keeps the head of the form in view and shows the form as what it is,
 * `(func $fib …)`: the body goes, and the parenthesis that closes it stays where a reader
 * expects to find it rather than on a line of its own.
 */
const folding = foldService.of((state, lineStart, lineEnd) => {
    const close = closes(state.doc, lineStart);

    if (close === null || close <= lineEnd) return null;

    return { from: lineEnd, to: close };
});

/**
 * The offset of the parenthesis closing the form that opens on the line beginning at `start`,
 * or `null` when the line opens no form, or opens one that never closes.
 *
 * A form is opened by a parenthesis on the line it starts at: a line that has not opened one by
 * the time it ends is a line that only continues what a line before it opened, and a line whose
 * first parenthesis is the one of a later line is a line of a body --- `local.get $n` above a
 * `(ref i31)` --- which is nothing to fold.
 *
 * The scan reads the text the way the tokenizer does: a parenthesis inside a string or a comment
 * is not a parenthesis.
 */
function closes(doc: Text, start: number): number | null {
    const text = doc.sliceString(start);
    let depth = 0;
    let comment = 0;
    let quoted = false;

    for (let at = 0; at < text.length; at++) {
        const char = text[at];

        if (char === "\n" && depth === 0) return null;

        if (comment > 0) {
            if (text.startsWith("(;", at)) {
                comment += 1;
                at += 1;
            } else if (text.startsWith(";)", at)) {
                comment -= 1;
                at += 1;
            }

            continue;
        }

        if (quoted) {
            if (char === "\\") at += 1;
            else if (char === '"') quoted = false;

            continue;
        }

        if (text.startsWith(";;", at)) {
            // A line comment runs to the end of its line, and a line that only comments opens
            // nothing; a form that is already open closes after the comment.
            if (depth === 0) return null;

            const end = text.indexOf("\n", at);

            // A comment that runs to the end of the file hides the rest of it.
            if (end === -1) return null;

            at = end;
            continue;
        }

        if (text.startsWith("(;", at)) {
            // `(;)` is a comment of nothing: the `;` of the opening is the one of the closing.
            if (text[at + 2] === ")") {
                at += 2;
                continue;
            }

            comment = 1;
            at += 1;
            continue;
        }

        if (char === '"') {
            quoted = true;
            continue;
        }

        if (char === "(") {
            depth += 1;
        } else if (char === ")") {
            // A close with nothing open before it continues a form rather than opening one.
            if (depth === 0) return null;

            depth -= 1;

            if (depth === 0) return start + at;
        }
    }

    return null;
}

/** The language of the viewer, ready to be handed to a view. */
export function watLanguageSupport(): LanguageSupport {
    return new LanguageSupport(wat, [syntaxHighlighting(codeStyle), folding]);
}
