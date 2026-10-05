/**
 * A ZIP archive of a build, written where the build is and read where it comes back ([ZIP]).
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
 * What is written here is read back by the browser check, which asserts the paths a build
 * holds, and by `mlkc-cli`, which runs a build with no directory to unpack it into. All of
 * them agree on one shape: a local header per file, stored contents, and a directory at the
 * end. An entry that is compressed is refused rather than guessed at, because a build that
 * came from anywhere else is a directory of files.
 *
 * [zip]: https://en.wikipedia.org/wiki/ZIP_(file_format)
 */

/** One file of an archive: the path it is written under, and the bytes written there. */
export interface ArchivedFile {
    /** The path of the file inside the archive, with `/` between its directories. */
    path: string;

    /** The contents of the file. */
    bytes: Uint8Array;
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
        stored.reduce(
            (all, it) => all + 30 + it.path.length + it.bytes.length,
            0,
        ) +
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

/** The signature of a local header, which every file of an archive begins with. */
const LOCAL = 0x04034b50;

/** The signature of a line of the directory at the end of an archive. */
const CENTRAL = 0x02014b50;

/** The signature of the end of a directory: the last thing an archive holds. */
const END = 0x06054b50;

/**
 * The files a ZIP archive holds, read off the directory at its end.
 *
 * The paths and the bytes are what a check reads of a build; the order is the order of the
 * entries in the directory, which is the order they were written in. A comment after the
 * directory is not one, so the end is searched for rather than assumed, the way the reader of
 * the CLI searches for it.
 */
export function readArchive(bytes: Uint8Array): ArchivedFile[] {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const end = endOf(view);
    const count = numberAt(view, end + 10, 2);
    const files: ArchivedFile[] = [];
    let at = numberAt(view, end + 16, 4);

    for (let index = 0; index < count; index++) {
        if (numberAt(view, at, 4) !== CENTRAL)
            throw new Error(`entry ${index} of the archive is not a header`);

        const method = numberAt(view, at + 10, 2);
        const size = numberAt(view, at + 24, 4);
        const name = numberAt(view, at + 28, 2);
        const extra = numberAt(view, at + 30, 2);
        const comment = numberAt(view, at + 32, 2);
        const local = numberAt(view, at + 42, 4);
        const path = new TextDecoder().decode(
            bytes.subarray(at + 46, at + 46 + name),
        );

        if (method !== 0)
            throw new Error(
                `the file \`${path}\` of the archive is compressed, and a build is stored ` +
                    "as it is: unpack the archive, or write it again without compression",
            );

        files.push({ path, bytes: contentsOf(bytes, view, local, size, path) });
        at += 46 + name + extra + comment;
    }

    return files;
}

/**
 * Where the directory of the archive ends, read from the end of it: a comment may stand after
 * it, and a signature inside a file of the archive is not one.
 */
function endOf(view: DataView): number {
    if (view.byteLength < 22)
        throw new Error("the archive is shorter than its end");

    const last = view.byteLength - 22;
    const from = Math.max(0, view.byteLength - 22 - 0xffff);

    for (let at = last; at >= from; at--) {
        if (numberAt(view, at, 4) === END) return at;
    }

    throw new Error("the archive has no directory at its end");
}

/**
 * The bytes of one file, read where its local header says they stand.
 *
 * A local header holds its own name and extra field again, and they need not be as long as the
 * ones of the directory: what says where the bytes begin is the header of the file itself.
 */
function contentsOf(
    bytes: Uint8Array,
    view: DataView,
    at: number,
    size: number,
    path: string,
): Uint8Array {
    if (numberAt(view, at, 4) !== LOCAL)
        throw new Error(`the file \`${path}\` of the archive is not a header`);

    const name = numberAt(view, at + 26, 2);
    const extra = numberAt(view, at + 28, 2);
    const from = at + 30 + name + extra;

    if (from + size > view.byteLength)
        throw new Error(`the archive ends in the middle of \`${path}\``);

    return bytes.subarray(from, from + size);
}

/**
 * The number at `at` of an archive, which `width` says how to read: little end first, and
 * checked against the end of the archive rather than left to the view to refuse.
 */
function numberAt(view: DataView, at: number, width: 2 | 4): number {
    if (at < 0 || at + width > view.byteLength)
        throw new Error("the archive ends in the middle of it");

    return width === 2 ? view.getUint16(at, true) : view.getUint32(at, true);
}
