import type { PaletteCommand } from '#/palette/palette_command';

export const MAXIMUM_PALETTE_RESULTS = 50;

const COMBINING_MARKS = /\p{Diacritic}/gu;
const WHITESPACE = /\s+/u;

function normalizeText(text: string): string {
	return text.normalize('NFD').replace(COMBINING_MARKS, '').toLowerCase();
}

/**
 * The commands whose label or category contains every word of the query,
 * ignoring case and accents, at most `MAXIMUM_PALETTE_RESULTS`.
 */
export function filterPaletteCommands(
	commands: readonly PaletteCommand[],
	query: string
): PaletteCommand[] {
	const words = normalizeText(query)
		.split(WHITESPACE)
		.filter((word) => word !== '');
	return commands
		.filter((command) => {
			const searchable = normalizeText(`${command.label} ${command.category}`);
			return words.every((word) => searchable.includes(word));
		})
		.slice(0, MAXIMUM_PALETTE_RESULTS);
}
