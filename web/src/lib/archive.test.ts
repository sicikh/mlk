import { readFile } from "node:fs/promises";

import { describe, expect, it } from "vitest";

import { archive, readArchive } from "./archive";

const ASCIIDOC = (value: string) => new TextEncoder().encode(value);

/** The bytes of an archive a zipper other than this one wrote. */
async function fixture(name: string): Promise<Uint8Array> {
    const path = new URL(`./fixtures/${name}`, import.meta.url);

    return new Uint8Array(await readFile(path));
}

/** What an archive holds, as the paths in the order it holds them. */
function pathsIn(bytes: Uint8Array): string[] {
    return readArchive(bytes).map((it) => it.path);
}

/** The bytes of one file of an archive. */
function fileIn(bytes: Uint8Array, path: string): Uint8Array {
    const file = readArchive(bytes).find((it) => it.path === path);

    if (!file) throw new Error(`the archive holds no \`${path}\``);

    return file.bytes;
}

describe("readArchive", () => {
    it("reads back what this writer wrote, paths and bytes", () => {
        const built = archive([
            {
                path: "app/main.wasm",
                bytes: new Uint8Array([0x00, 0x61, 0x73, 0x6d]),
            },
            { path: "manifest.json", bytes: ASCIIDOC('{"project":"app"}') },
            // A path and text of another script, which the writer encodes as UTF-8.
            { path: "std/é.mlk", bytes: ASCIIDOC("module std::e\n") },
        ]);

        expect(pathsIn(built)).toEqual([
            "app/main.wasm",
            "manifest.json",
            "std/é.mlk",
        ]);
        expect(Array.from(fileIn(built, "app/main.wasm"))).toEqual([
            0x00, 0x61, 0x73, 0x6d,
        ]);
        expect(new TextDecoder().decode(fileIn(built, "manifest.json"))).toBe(
            '{"project":"app"}',
        );
        expect(new TextDecoder().decode(fileIn(built, "std/é.mlk"))).toBe(
            "module std::e\n",
        );
    });

    it("reads an archive of no files", () => {
        expect(pathsIn(archive([]))).toEqual([]);
    });

    it("reads an archive a zipper other than this one wrote", async () => {
        const bytes = await fixture("written.zip");

        expect(pathsIn(bytes)).toEqual([
            "app/main.wasm",
            "manifest.json",
            "std/é.mlk",
        ]);
        expect(Array.from(fileIn(bytes, "app/main.wasm"))).toEqual([
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        ]);
        expect(new TextDecoder().decode(fileIn(bytes, "std/é.mlk"))).toBe(
            "module std::e\n",
        );
    });

    it("reads an archive whose directory is followed by a comment", async () => {
        // The fixture carries a comment after its directory, which the end of an archive is
        // searched for past rather than assumed at the end of the bytes.
        const bytes = await fixture("written.zip");

        expect(pathsIn(bytes)).toHaveLength(3);
    });

    it("refuses an entry that is compressed, and says which", async () => {
        const bytes = await fixture("deflated.zip");

        expect(() => readArchive(bytes)).toThrow(
            /`app\/main.wasm`.+compressed/,
        );
    });

    it("refuses bytes that are shorter than the end of an archive", () => {
        expect(() => readArchive(new Uint8Array([1, 2, 3]))).toThrow(
            /shorter than its end/,
        );
    });

    it("refuses an archive whose directory is missing", () => {
        expect(() => readArchive(new Uint8Array(32))).toThrow(
            /no directory at its end/,
        );
    });
});
