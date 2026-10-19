import { findReservedTagProblem } from '#/shared/reserved_tag_problem';

const TAG_SEPARATOR = ',';
const WHITESPACE_PATTERN = /\s/u;

export const EMPTY_TAG_MESSAGE = 'Saisissez un nom de tag.';
export const SEPARATOR_TAG_MESSAGE = 'Un seul tag, sans virgule.';
export const WHITESPACE_TAG_MESSAGE = "Le tag ne peut pas contenir d'espace.";
export const UNCHANGED_TAG_MESSAGE = 'Choisissez un nom différent.';

/** The French error of a new tag name, or `null` when it can be sent to the daemon. */
export function validateTagName(
	text: string,
	currentTag: string
): string | null {
	const name = text.trim();
	if (name === '') return EMPTY_TAG_MESSAGE;
	if (name.includes(TAG_SEPARATOR)) return SEPARATOR_TAG_MESSAGE;
	if (WHITESPACE_PATTERN.test(name)) return WHITESPACE_TAG_MESSAGE;
	const reservedProblem = findReservedTagProblem(name);
	if (reservedProblem !== null) return reservedProblem;
	if (name === currentTag) return UNCHANGED_TAG_MESSAGE;
	return null;
}
