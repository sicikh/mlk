/**
 * The compiler counts text in bytes; a browser counts it in UTF-16 code units.
 *
 * Everything the compiler hands over — the range of a token, the place a diagnostic points at —
 * is in bytes, because that is what the text is. Everything a JavaScript string
 * and an editor say is in UTF-16 code units. This is the one place that translates.
 */

/** How many bytes a character takes in UTF-8. */
function width(character: string): number {
	const point = character.codePointAt(0) ?? 0;

	if (point < 0x80) return 1;
	if (point < 0x800) return 2;
	if (point < 0x10000) return 3;

	return 4;
}

/** How many bytes a text takes in UTF-8. */
export function byteLength(text: string): number {
	let bytes = 0;

	for (const character of text) bytes += width(character);

	return bytes;
}

/** A map from the byte offsets of a text to the places a string counts. */
export interface Offsets {
	/** Where a byte of the text sits in it, counted the way a string counts. */
	at(bytes: number): number;
}

function offsetsOfText(text: string): Offsets {
	const map = new Uint32Array(byteLength(text) + 1);
	let at = 0;
	let units = 0;

	for (const character of text) {
		const size = width(character);

		// A byte in the middle of a character belongs to the character it starts.
		for (let i = 0; i < size; i++) map[at + i] = units;

		at += size;
		units += character.length;
	}

	map[map.length - 1] = units;

	return { at: (bytes) => map[Math.min(Math.max(bytes, 0), map.length - 1)] };
}

let cached: { text: string; offsets: Offsets } | undefined;

/**
 * The map of a text, kept for the text it was made from: a page reads one buffer at a time,
 * and every reader of it asks the same question.
 */
export function offsetsOf(text: string): Offsets {
	if (cached?.text !== text) cached = { text, offsets: offsetsOfText(text) };

	return cached.offsets;
}

/** Where a byte of a text sits in it, counted the way a string counts. */
export function utf16At(text: string, bytes: number): number {
	return offsetsOf(text).at(bytes);
}
