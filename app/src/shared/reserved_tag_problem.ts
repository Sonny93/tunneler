const PORT_SELECTOR_SEPARATOR = ':';
const ONLY_DIGITS_PATTERN = /^\d+$/u;

export const COLON_TAG_MESSAGE =
	'Le tag ne peut pas contenir « : » (réservé à la forme hôte:port de la ligne de commande).';
export const DIGITS_TAG_MESSAGE =
	'Le tag ne peut pas être composé uniquement de chiffres (la ligne de commande y verrait un port).';

/** The French error of a tag the CLI would read as a port selector, or `null`. */
export function findReservedTagProblem(tag: string): string | null {
	if (tag.includes(PORT_SELECTOR_SEPARATOR)) return COLON_TAG_MESSAGE;
	if (ONLY_DIGITS_PATTERN.test(tag)) return DIGITS_TAG_MESSAGE;
	return null;
}
