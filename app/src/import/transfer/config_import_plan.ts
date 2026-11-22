import type { ConfigImportPlanDto } from '#/daemon/protocol';

export const NO_HOST_ALIAS = '';

/** The entries to check when a plan arrives: the importable ones without a risk. */
export function getDefaultSelection(
	plan: Readonly<ConfigImportPlanDto>
): Set<number> {
	return new Set(
		plan.entries.flatMap((entry) =>
			entry.selectedByDefault ? [entry.index] : []
		)
	);
}

/**
 * The hosts that get a remap field. A host leaves `unknownHosts` once it is
 * mapped, so the fields already shown stay, or the one just used would vanish
 * under the user's hands.
 */
export function mergeRemapHosts(
	shownHosts: readonly string[],
	plan: Readonly<ConfigImportPlanDto>
): string[] {
	return [...new Set([...shownHosts, ...plan.unknownHosts])];
}

/** A host map with `alias` for `fileHost`, or without the host when `alias` is empty. */
export function withHostAlias(
	hostMap: Readonly<Record<string, string>>,
	fileHost: string,
	alias: string
): Record<string, string> {
	const others = Object.fromEntries(
		Object.entries(hostMap).filter(([host]) => host !== fileHost)
	);
	return alias === NO_HOST_ALIAS ? others : { ...others, [fileHost]: alias };
}
