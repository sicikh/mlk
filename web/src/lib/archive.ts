/**
 * A ZIP archive of a build, written where the build is ([ZIP]).
 *
 * A browser has no file system, and a download cannot make a folder: what a host can hand a
 * person is one file, and an archive is the one file a project's shape survives in --- every
 * module under the path its canonical name stands for, so `app::main` is `app/main.wasm` and
 * the modules of a project lie in the folder of the project.
 *
 * The format is written here rather than taken from a package: an archive is a handful of
 * files whose sizes are known before anything is written, and every one of them is stored as
 * it is --- the method the specification calls "store". A module of WASM is compact already,
 * and what the archive is for is structure rather than bytes.
 *
 * [zip]: https://en.wikipedia.org/wiki/ZIP_(file_format)
 */

/** One file of an archive: the path it is written under, and the bytes written there. */
export interface ArchivedFile {
    /** The path of the file inside the archive, with `/` between its directories. */
    path: string;

    /** The contents of the file. */
    bytes: Uint8Array | number[];
}

/**
 * The date every entry is stamped with: the first of January, 1980, where the format counts
 * from. The moment of a build is not a fact the archive is meant to carry, and a fixed date
 * is what makes two builds of the same bytes the same file.
 */
const STAMPED = (1 << 5) | 1;

/** The table of CRC-32, made once for every file of every archive. */
const CRC = (() => {
    const table = new Uint32Array(256);

    for (let n = 0; n < 256; n++) {
        let value = n;

        for (let bit = 0; bit < 8; bit++)
            value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;

        table[n] = value >>> 0;
    }

    return table;
})();

/** The checksum every file of a ZIP carries, over the bytes of the file. */
function crc32(bytes: Uint8Array): number {
    let crc = 0xffffffff;

    for (const byte of bytes) crc = CRC[(crc ^ byte) & 0xff] ^ (crc >>> 8);

    return (crc ^ 0xffffffff) >>> 0;
}

/**
 * The bytes of a ZIP archive holding every file of `files`, stored as they are.
 *
 * The order of `files` is the order of the entries in the archive. A path may name a
 * directory that no entry names: a reader makes the folder from the path of a file, which is
 * why nothing writes an entry for a directory of its own.
 */
export function archive(files: ArchivedFile[]): Uint8Array {
    const stored = files.map((file) => {
        const path = new TextEncoder().encode(file.path);
        const bytes = new Uint8Array(file.bytes);

        return { path, bytes, crc: crc32(bytes) };
    });

    // An archive is its files and then a directory of the same files at its end: every entry
    // is written twice, once with its contents and once as a line of that directory, which is
    // where a reader looks for an entry without walking everything written before it.
    const size =
        stored.reduce((all, it) => all + 30 + it.path.length + it.bytes.length, 0) +
        stored.reduce((all, it) => all + 46 + it.path.length, 0) +
        22;
    const out = new Uint8Array(size);
    const view = new DataView(out.buffer);
    const offsets: number[] = [];
    let at = 0;

    for (const it of stored) {
        offsets.push(at);

        // The header of a file, then its path, then its bytes.
        view.setUint32(at, 0x04034b50, true);
        view.setUint16(at + 4, 20, true);
        view.setUint16(at + 6, 0x0800, true);
        view.setUint16(at + 8, 0, true);
        view.setUint16(at + 10, 0, true);
        view.setUint16(at + 12, STAMPED, true);
        view.setUint32(at + 14, it.crc, true);
        view.setUint32(at + 18, it.bytes.length, true);
        view.setUint32(at + 22, it.bytes.length, true);
        view.setUint16(at + 26, it.path.length, true);
        view.setUint16(at + 28, 0, true);
        out.set(it.path, at + 30);
        out.set(it.bytes, at + 30 + it.path.length);
        at += 30 + it.path.length + it.bytes.length;
    }

    const directory = at;

    for (const [index, it] of stored.entries()) {
        // A line of the directory: the facts of the header again, and where the file stands.
        view.setUint32(at, 0x02014b50, true);
        view.setUint16(at + 4, 20, true);
        view.setUint16(at + 6, 20, true);
        view.setUint16(at + 8, 0x0800, true);
        view.setUint16(at + 10, 0, true);
        view.setUint16(at + 12, 0, true);
        view.setUint16(at + 14, STAMPED, true);
        view.setUint32(at + 16, it.crc, true);
        view.setUint32(at + 20, it.bytes.length, true);
        view.setUint32(at + 24, it.bytes.length, true);
        view.setUint16(at + 28, it.path.length, true);
        view.setUint16(at + 30, 0, true);
        view.setUint16(at + 32, 0, true);
        view.setUint16(at + 34, 0, true);
        view.setUint16(at + 36, 0, true);
        view.setUint32(at + 38, 0, true);
        view.setUint32(at + 42, offsets[index], true);
        out.set(it.path, at + 46);
        at += 46 + it.path.length;
    }

    // And where the directory ends: where it stands, how long it is, and what it holds.
    view.setUint32(at, 0x06054b50, true);
    view.setUint16(at + 4, 0, true);
    view.setUint16(at + 6, 0, true);
    view.setUint16(at + 8, stored.length, true);
    view.setUint16(at + 10, stored.length, true);
    view.setUint32(at + 12, at - directory, true);
    view.setUint32(at + 16, directory, true);
    view.setUint16(at + 20, 0, true);

    return out;
}
