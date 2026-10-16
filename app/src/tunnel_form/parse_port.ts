export const MIN_PORT = 1;
export const MAX_PORT = 65_535;

const INTEGER_PATTERN = /^\d+$/;

/** The port written in `text`, or `null` when it is not an integer in 1–65535. */
export function parsePort(text: string): number | null {
	const trimmed = text.trim();
	if (!INTEGER_PATTERN.test(trimmed)) return null;
	const port = Number(trimmed);
	return port >= MIN_PORT && port <= MAX_PORT ? port : null;
}
