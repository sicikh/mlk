import { describe, expect, it } from "vitest";

import type { Diagnostic, Label } from "./driver";
import { codeOf, mainLabel, rangeOf, severityOf } from "./diagnostics";

/** A label pointing at a place of a buffer, of which a test says what it is about. */
function label(parts: Partial<Label> = {}): Label {
    return {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
        primary: false,
        message: "",
        ...parts,
    };
}

/** A diagnostic of the parser, of which a test says what it is about. */
function diagnostic(parts: Partial<Diagnostic> = {}): Diagnostic {
    return {
        level: "error",
        category: "parser",
        categoryCode: "02",
        code: "01",
        message: "",
        labels: [],
        notes: [],
        ...parts,
    };
}

describe("codeOf", () => {
    it("leads with the level and follows with the category and the kind", () => {
        expect(codeOf(diagnostic())).toBe("E0201");
    });

    it("names the levels the way rustc does", () => {
        expect(codeOf(diagnostic({ level: "warning" }))).toBe("W0201");
        expect(codeOf(diagnostic({ level: "note" }))).toBe("N0201");
        expect(codeOf(diagnostic({ level: "help" }))).toBe("H0201");
    });

    it("reads a level it does not know as an error", () => {
        expect(codeOf(diagnostic({ level: "bug" }))).toBe("E0201");
    });
});

describe("severityOf", () => {
    it("reads the levels of the compiler the way an editor names them", () => {
        expect(severityOf("error")).toBe("error");
        expect(severityOf("warning")).toBe("warning");
        expect(severityOf("note")).toBe("info");
        expect(severityOf("help")).toBe("hint");
    });

    it("reads a level it does not know as an error", () => {
        expect(severityOf("bug")).toBe("error");
    });
});

describe("mainLabel", () => {
    it("takes the label the diagnostic is about, wherever it stands", () => {
        const about = label({ start: 4, primary: true });
        const also = label({ start: 0 });

        expect(mainLabel(diagnostic({ labels: [also, about] }))).toBe(about);
    });

    it("takes the first when nothing is the one it is about", () => {
        const first = label({ start: 0 });
        const second = label({ start: 4 });

        expect(mainLabel(diagnostic({ labels: [first, second] }))).toBe(first);
    });

    it("takes nothing of a diagnostic that points nowhere", () => {
        expect(mainLabel(diagnostic())).toBeUndefined();
    });
});

describe("rangeOf", () => {
    it("counts where a label points the way a string counts", () => {
        expect(rangeOf(label({ start: 4, end: 7 }), "fun main")).toEqual({
            from: 4,
            to: 7,
        });
    });

    it("counts the bytes of a character before the label", () => {
        // `é` is two bytes and one unit: what follows it moves by one.
        expect(rangeOf(label({ start: 2, end: 3 }), "é x")).toEqual({
            from: 1,
            to: 2,
        });
    });

    it("hangs a range that ends before it starts at its start", () => {
        expect(rangeOf(label({ start: 4, end: 2 }), "fun main")).toEqual({
            from: 4,
            to: 4,
        });
    });
});
