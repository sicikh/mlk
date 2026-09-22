/**
 * What a diagnostic of the compiler looks like where a person reads it.
 *
 * The compiler says what it found and where ([ADR-0002]); this decides what
 * that is called in an editor, which is a question of presentation and nobody else's.
 *
 * [ADR-0002]: https://github.com/sicikh/mlk/blob/main/docs/adr/0002-lossless-syntax-tree.md
 */
import type { Diagnostic, Label } from './driver';
import { utf16At } from './offsets';

/** The letter a level is known by, the way rustc spells its codes. */
function letterOf(level: string): string {
	switch (level) {
		case 'warning':
			return 'W';
		case 'note':
			return 'N';
		case 'help':
			return 'H';
		default:
			return 'E';
	}
}

/**
 * The code of a diagnostic, as a person reads it: `E0001`.
 *
 * The compiler holds the kind of a diagnostic as a short code within its category
 * ([ADR-0005]), which is what the CLI prints as `error[0201]`.
 * Here the level leads and the kind's number follows.
 *
 * [ADR-0005]: https://github.com/sicikh/mlk/blob/main/docs/adr/0005-compiler-pipeline.md
 */
export function codeOf(diagnostic: Diagnostic): string {
	return `${letterOf(diagnostic.level)}${diagnostic.code.padStart(4, '0')}`;
}

/** How a level of the compiler is called in an editor. */
export function severityOf(level: string): 'error' | 'warning' | 'info' | 'hint' {
	switch (level) {
		case 'warning':
			return 'warning';
		case 'note':
			return 'info';
		case 'help':
			return 'hint';
		default:
			return 'error';
	}
}

/** The label a diagnostic is about, or the first it has. */
export function mainLabel(diagnostic: Diagnostic): Label | undefined {
	return diagnostic.labels.find((it) => it.primary) ?? diagnostic.labels.at(0);
}

/** Where a label points, counted the way a string counts. */
export function rangeOf(label: Label, text: string): { from: number; to: number } {
	const from = utf16At(text, label.start);

	return { from, to: Math.max(from, utf16At(text, label.end)) };
}
