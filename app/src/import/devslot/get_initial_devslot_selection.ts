import {
	isPortPreselected,
	type DevslotPortEntry,
} from '#/import/devslot/devslot_ports';
import { findLocalPortProblems } from '#/import/devslot/find_devslot_problems';

/**
 * The ports checked at first: the preselected ones whose local port is usable.
 * Conflicts with tunnels are only known once the daemon answered, so the form
 * unchecks those then.
 */
export function getInitialDevslotSelection(
	entries: readonly DevslotPortEntry[]
): Set<string> {
	const problems = findLocalPortProblems({ entries, localPorts: {} });
	return new Set(
		entries
			.filter(isPortPreselected)
			.map(({ key }) => key)
			.filter((key) => !problems.has(key))
	);
}
