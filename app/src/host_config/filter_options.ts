import type { ResolvedHostOptionDto } from '#/daemon/protocol';

/** The options whose key or value contains `query`, ignoring case; a blank query keeps all. */
export function filterOptions(
	options: readonly ResolvedHostOptionDto[],
	query: string
): readonly ResolvedHostOptionDto[] {
	const normalizedQuery = query.trim().toLowerCase();
	if (normalizedQuery === '') return options;
	return options.filter(
		({ key, value }) =>
			key.toLowerCase().includes(normalizedQuery) ||
			value.toLowerCase().includes(normalizedQuery)
	);
}
