import { EditorState } from "@codemirror/state";
import { getStyleTags, tags, type Tag } from "@lezer/highlight";
import { describe, expect, it } from "vitest";

import { mlkLanguage, mlkLanguageSupport } from "./highlight";

/** The program the editor opens with, which the grammar has to know. */
const STARTER = `fun fib(n : Int) : Int =
    fib-aux(n, 0, 1)

fun fib-aux(n : Int, a : Int, b : Int) : Int =
    if n == 0 then
        a
    else
        fib-aux(n - 1, b, a + b)

#[entry]
pub fun main() : Unit =
    print-int(fib(5))
`;

/** The tags the deepest node covering `at` is painted with. */
function tagsAt(source: string, at: number) {
    const tree = mlkLanguage.parser.parse(source);
    let found: readonly Tag[] = [];

    tree.iterate({
        enter(node) {
            if (node.from <= at && at < node.to) {
                const style = getStyleTags(node);

                if (style) found = style.tags;
            }
        },
    });

    return found;
}

/** Where a word of a source stands, as a word rather than a piece of another. */
function wordAt(source: string, word: string) {
    const at = source.search(new RegExp(`\\b${word}\\b`));

    if (at === -1) throw new Error(`the source holds no \`${word}\``);

    return at;
}

describe("the colours of the language", () => {
    it("paints the keywords it reads out of names", () => {
        const source = `#[entry]\npub fun main() : Unit =\n    let x = 1 in\n    x\n`;

        for (const word of ["pub", "fun", "let", "in"])
            expect(tagsAt(source, wordAt(source, word))).toContain(
                tags.keyword,
            );
    });

    it("paints the words a choice is written with", () => {
        const source = `fun pick(flag: Bool) : Int =\n    if flag then 1 else 2\n`;

        for (const word of ["if", "then", "else"])
            expect(tagsAt(source, wordAt(source, word))).toContain(
                tags.keyword,
            );
    });

    it("paints use and as, for an import renamed where it is written", () => {
        const source = `use project::data::core as data\n\npub fun main() : Unit =\n    data.start()\n`;

        expect(tagsAt(source, source.indexOf("use"))).toContain(tags.keyword);
        expect(tagsAt(source, source.indexOf("as"))).toContain(tags.keyword);
    });

    it("paints a type where a type belongs, and the path it is written as", () => {
        const source = `fun main(x: core::Int) : Unit = 0\n`;

        expect(tagsAt(source, wordAt(source, "core"))).toContain(tags.typeName);
        expect(tagsAt(source, wordAt(source, "Int"))).toContain(tags.typeName);
        expect(tagsAt(source, wordAt(source, "Unit"))).toContain(tags.typeName);
    });

    it("paints a name the colour of text", () => {
        const source = `fun main() : Unit = 0\n`;

        // A page of code is mostly names, and painting them all makes the rest harder
        // to read: only what the grammar can tell apart is painted.
        expect(tagsAt(source, source.indexOf("main"))).toEqual([]);
        expect(tagsAt(source, source.indexOf("0"))).toContain(tags.number);
    });

    it("paints the literals of the language", () => {
        const source = `fun f() : Bool =\n    let x = 5 in\n    let y = "text" in\n    true\n`;

        expect(tagsAt(source, source.indexOf("5"))).toContain(tags.number);
        expect(tagsAt(source, source.indexOf('"text"'))).toContain(tags.string);
        expect(tagsAt(source, source.indexOf("true"))).toContain(tags.bool);
    });

    it("paints an attribute whole, the name inside it with it", () => {
        const source = `#[entry]\nfun main() : Unit = 0\n`;

        expect(tagsAt(source, source.indexOf("#[entry]"))).toContain(
            tags.annotation,
        );
        // The `/...` of the attribute says the tag reaches the words inside it: what the
        // `#` introduces is a word of the language, not a name the code gives to something.
        expect(tagsAt(source, wordAt(source, "entry"))).toContain(
            tags.annotation,
        );
    });

    it("paints a comment to the end of the line, and one that nests", () => {
        const line = `fun main() : Unit = 0 // note\n`;

        expect(tagsAt(line, line.indexOf("//"))).toContain(tags.comment);

        const block = `/* a\n/* b */\nc */\nfun main() : Unit = 0\n`;

        expect(tagsAt(block, block.indexOf("/*"))).toContain(tags.comment);
        expect(tagsAt(block, block.indexOf("fun"))).toContain(tags.keyword);
    });

    it("parses the program the editor opens with, without an error node", () => {
        const tree = mlkLanguage.parser.parse(STARTER);
        const errors: string[] = [];

        tree.iterate({
            enter(node) {
                if (node.name === "⚠") errors.push(node.name);
            },
        });

        expect(tree.topNode.name).toBe("ModuleRoot");
        expect(errors).toEqual([]);
    });

    it("says which tokens a comment of the language is written with", () => {
        const state = EditorState.create({
            doc: "",
            extensions: [mlkLanguageSupport()],
        });

        expect(state.languageDataAt("commentTokens", 0)).toEqual([
            { line: "//", block: { open: "/*", close: "*/" } },
        ]);
    });
});
