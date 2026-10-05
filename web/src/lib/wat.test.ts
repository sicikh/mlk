import { EditorState } from "@codemirror/state";
import { ensureSyntaxTree, foldable } from "@codemirror/language";
import { getStyleTags, tags, type Tag } from "@lezer/highlight";
import { describe, expect, it } from "vitest";

import { watLanguageSupport } from "./wat";

/** The tree of a text of the format, parsed the way a view parses it. */
function treeOf(doc: string) {
    const state = EditorState.create({
        doc,
        extensions: [watLanguageSupport()],
    });
    const tree = ensureSyntaxTree(state, doc.length, 1000);

    if (!tree) throw new Error("the wat language did not parse the text");

    return tree;
}

/** The tags the deepest node covering `at` is painted with. */
function tagsAt(doc: string, at: number) {
    let found: readonly Tag[] = [];

    treeOf(doc).iterate({
        enter(node) {
            if (node.from <= at && at < node.to) {
                const style = getStyleTags(node);

                if (style) found = style.tags;
            }
        },
    });

    return found;
}

/** What a fold of the line at `line` covers, or null when the line folds nothing. */
function foldAt(doc: string, line: number) {
    const state = EditorState.create({
        doc,
        extensions: [watLanguageSupport()],
    });
    let start = 0;

    for (let index = 0; index < line; index++)
        start = doc.indexOf("\n", start) + 1;

    const newline = doc.indexOf("\n", start);
    const end = newline === -1 ? doc.length : newline;

    return foldable(state, start, end);
}

describe("the tokens of the format", () => {
    const doc = `(module $app (func $f (result i32) i32.const 0x2a))`;

    it("paints a word of the format as a keyword", () => {
        for (const word of ["module", "func", "result", "i32.const"])
            expect(tagsAt(doc, doc.indexOf(word))).toContain(tags.keyword);
    });

    it("paints a value type as a type, and an instruction named like one as a keyword", () => {
        expect(tagsAt(doc, doc.indexOf("i32)"))).toContain(tags.typeName);
        expect(tagsAt(doc, doc.indexOf("i32.const"))).toContain(tags.keyword);
    });

    it("paints what a module names a thing of its own as a name", () => {
        expect(tagsAt(doc, doc.indexOf("$app"))).toContain(tags.variableName);
        expect(tagsAt(doc, doc.indexOf("$f"))).toContain(tags.variableName);
    });

    it("paints a number as a number", () => {
        expect(tagsAt(doc, doc.indexOf("0x2a"))).toContain(tags.number);
    });

    it("paints the numbers the format is written with", () => {
        const numbers = `(a -1.5 1_000 inf nan:0x1 0x1f)`;

        for (const number of ["-1.5", "1_000", "inf", "nan:0x1", "0x1f"])
            expect(tagsAt(numbers, numbers.indexOf(number))).toContain(
                tags.number,
            );
    });

    it("paints a quoted name as a string, and reads its escapes", () => {
        const escaped = `(data "a\\"b" func)`;

        expect(tagsAt(escaped, escaped.indexOf('"'))).toContain(tags.string);
        // The quote of the escape ends nothing: what follows the string is a word again.
        expect(tagsAt(escaped, escaped.indexOf("func"))).toContain(
            tags.keyword,
        );
    });

    it("paints a line comment to the end of the line", () => {
        const comment = `(module ;; ) not a paren\n  $f)`;

        expect(tagsAt(comment, comment.indexOf(";;"))).toContain(tags.comment);
        expect(tagsAt(comment, comment.indexOf("$f"))).toContain(
            tags.variableName,
        );
    });

    it("paints a block comment, which nests", () => {
        const nested = `(module (; a (; b ;) c ;) $f)`;

        expect(tagsAt(nested, nested.indexOf("(;"))).toContain(tags.comment);
        // Every opening of a comment inside is closed by a closing of its own.
        expect(tagsAt(nested, nested.indexOf("$f"))).toContain(
            tags.variableName,
        );
    });

    it("paints a block comment of nothing, which its own closing ends", () => {
        const empty = `(module (;) $f)`;

        expect(tagsAt(empty, empty.indexOf("(;"))).toContain(tags.comment);
        expect(tagsAt(empty, empty.indexOf("$f"))).toContain(tags.variableName);
    });

    it("paints a block comment that spans lines", () => {
        const spanned = `(module (; a\n  b ;) $f)`;

        expect(tagsAt(spanned, spanned.indexOf("(;"))).toContain(tags.comment);
        expect(tagsAt(spanned, spanned.indexOf("$f"))).toContain(
            tags.variableName,
        );
    });
});

describe("the folds of the format", () => {
    it("folds a form from the end of the line it opens on to its closing", () => {
        const doc = `(module $app\n  (func $f\n    i32.const 42\n  )\n)\n`;
        const module = foldAt(doc, 0);
        const func = foldAt(doc, 1);

        expect(module).toEqual({
            from: doc.indexOf("\n"),
            to: doc.lastIndexOf(")"),
        });
        expect(doc.slice(func?.from, func?.to)).toBe("\n    i32.const 42\n  ");
    });

    it("folds nothing on a line that opens no form", () => {
        const doc = `(module $f\n  i32.const 42\n)\n`;

        expect(foldAt(doc, 1)).toBeNull();
    });

    it("folds nothing on a line that closes a form", () => {
        const doc = `(module $f\n  i32.const 42\n)\n`;

        expect(foldAt(doc, 2)).toBeNull();
    });

    it("folds nothing on a line that only comments", () => {
        const doc = `(module $f\n  ;; (a form that is not one)\n)\n`;

        expect(foldAt(doc, 1)).toBeNull();
    });

    it("does not read a parenthesis of a comment as one", () => {
        const doc = `(module ;; ) not a paren\n)\n`;
        const module = foldAt(doc, 0);

        expect(module?.to).toBe(doc.lastIndexOf(")"));
    });

    it("does not read a parenthesis of a string as one", () => {
        const doc = `(data ")" more\n)\n`;
        const module = foldAt(doc, 0);

        expect(module?.to).toBe(doc.lastIndexOf(")"));
    });

    it("closes a comment that nests before the form it stands in", () => {
        const doc = `(module (; (; ;) ;)\n)\n`;

        expect(foldAt(doc, 0)?.to).toBe(doc.lastIndexOf(")"));
    });

    it("closes a comment of nothing before the form it stands in", () => {
        const doc = `(module (;) x\n)\n`;

        expect(foldAt(doc, 0)?.to).toBe(doc.lastIndexOf(")"));
    });

    it("folds nothing of a form that never closes", () => {
        const doc = `(module $f\n  i32.const 42\n`;

        expect(foldAt(doc, 0)).toBeNull();
    });

    it("closes a form inside a form at its own parenthesis", () => {
        const doc = `(a (b\n  c\n)\n)\n`;

        expect(foldAt(doc, 0)?.to).toBe(doc.lastIndexOf(")"));
        // The line of the closing of the inner form continues a form rather than opening one.
        expect(foldAt(doc, 2)).toBeNull();
    });
});
