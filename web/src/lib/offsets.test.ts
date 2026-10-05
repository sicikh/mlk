import { describe, expect, it } from "vitest";

import { byteLength, utf16At } from "./offsets";

describe("byteLength", () => {
    it("counts a text of ASCII a byte a character", () => {
        expect(byteLength("")).toBe(0);
        expect(byteLength("fun main")).toBe(8);
    });

    it("counts what UTF-8 says a character of another script takes", () => {
        expect(byteLength("é")).toBe(2);
        expect(byteLength("Ж")).toBe(2);
        expect(byteLength("中")).toBe(3);
        expect(byteLength("€")).toBe(3);
        // An emoji is one code point of four bytes, which is two code units of a string.
        expect(byteLength("😀")).toBe(4);
    });

    it("counts a mixed text, character by character", () => {
        expect(byteLength("a é 中 😀")).toBe(1 + 1 + 2 + 1 + 3 + 1 + 4);
    });
});

describe("utf16At", () => {
    it("maps ASCII one to one", () => {
        expect(utf16At("abc", 0)).toBe(0);
        expect(utf16At("abc", 1)).toBe(1);
        expect(utf16At("abc", 3)).toBe(3);
    });

    it("puts a byte in the middle of a character at the character it starts", () => {
        // `é` is two bytes and one code unit.
        expect(utf16At("é", 0)).toBe(0);
        expect(utf16At("é", 1)).toBe(0);
        expect(utf16At("é", 2)).toBe(1);
    });

    it("counts a code point of two units as two", () => {
        // `a😀b`: `a` is one byte, the emoji four bytes and two units, `b` one byte.
        const text = "a😀b";

        expect(utf16At(text, 0)).toBe(0);
        expect(utf16At(text, 1)).toBe(1);
        expect(utf16At(text, 4)).toBe(1);
        expect(utf16At(text, 5)).toBe(3);
        expect(utf16At(text, 6)).toBe(4);
    });

    it("hangs a byte outside the text at its nearest end", () => {
        expect(utf16At("abc", -1)).toBe(0);
        expect(utf16At("abc", 99)).toBe(3);
    });

    it("maps every text by its own map", () => {
        // The map is kept for the text it was made from: a question about another text
        // is not answered by the map of the one before it.
        expect(utf16At("é", 2)).toBe(1);
        expect(utf16At("中", 3)).toBe(1);
        expect(utf16At("é", 2)).toBe(1);
    });
});
