import type { ResolvedHostOptionDto } from '#/daemon/protocol';

/** One `key value` line per option, as `ssh -G` prints them. */
export function formatConfigLines(
	options: readonly ResolvedHostOptionDto[]
): string {
	return options.map(({ key, value }) => `${key} ${value}`).join('\n');
}
