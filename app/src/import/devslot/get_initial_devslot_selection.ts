import type { SettingsDto, TunnelDto } from '#/daemon/protocol';
import {
	isPortPreselected,
	type DevslotPortEntry,
} from '#/import/devslot/devslot_ports';
import { findDevslotProblems } from '#/import/devslot/find_devslot_problems';

/** The ports checked at first: the preselected ones that do not clash with anything. */
export function getInitialDevslotSelection(
	entries: readonly DevslotPortEntry[],
	tunnels: readonly Readonly<TunnelDto>[],
	settings: Readonly<SettingsDto>,
	host: string
): Set<string> {
	const candidates = new Set(
		entries.filter(isPortPreselected).map(({ key }) => key)
	);
	const problems = findDevslotProblems({
		entries,
		localPorts: {},
		selectedKeys: candidates,
		tunnels,
		settings,
		host,
	});
	return new Set([...candidates].filter((key) => problems.get(key) === null));
}
