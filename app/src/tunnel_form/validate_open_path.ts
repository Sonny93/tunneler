const WHITESPACE_PATTERN = /\s/u;
const CONTROL_CHARACTER_PATTERN = /\p{Cc}/u;

const MISSING_SLASH_MESSAGE = 'Le chemin doit commencer par « / ».';
const DOUBLE_SLASH_MESSAGE = 'Le chemin ne doit pas commencer par « // ».';
const FORBIDDEN_CHARACTER_MESSAGE =
	'Le chemin ne doit contenir ni espace ni caractère de contrôle.';

/** The French error of an open path, or `null` when it is valid; blank means the default `/`. Same rules as the engine. */
export function validateOpenPath(text: string): string | null {
	const path = text.trim();
	if (path === '') return null;
	if (!path.startsWith('/')) return MISSING_SLASH_MESSAGE;
	if (path.startsWith('//')) return DOUBLE_SLASH_MESSAGE;
	if (WHITESPACE_PATTERN.test(path) || CONTROL_CHARACTER_PATTERN.test(path)) {
		return FORBIDDEN_CHARACTER_MESSAGE;
	}
	return null;
}
