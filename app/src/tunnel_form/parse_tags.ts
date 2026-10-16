const TAGS_SEPARATOR = ',';

/** Trimmed, deduplicated tags in order of appearance; empty entries are dropped. */
export function parseTags(text: string): string[] {
	const tags = text
		.split(TAGS_SEPARATOR)
		.map((tag) => tag.trim())
		.filter((tag) => tag !== '');
	return [...new Set(tags)];
}
