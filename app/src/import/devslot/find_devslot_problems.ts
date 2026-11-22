import type { SettingsDto, TunnelDto } from '#/daemon/protocol';
import {
	isPortTunnelable,
	type DevslotPortEntry,
} from '#/import/devslot/devslot_ports';
import {
	findImportConflicts,
	type ImportConflict,
} from '#/import/find_import_conflicts';
import { parsePort } from '#/tunnel_form/parse_port';

export const DEVSLOT_LISTEN_ADDRESS = '127.0.0.1';

export type DevslotPortProblem =
	| { kind: 'invalidLocalPort' }
	| { kind: 'localPortBusy'; port: number }
	| ImportConflict;

export type DevslotLocalPorts = Readonly<Record<string, string>>;

type FindDevslotProblemsInput = {
	entries: readonly DevslotPortEntry[];
	localPorts: DevslotLocalPorts;
	selectedKeys: ReadonlySet<string>;
	tunnels: readonly Readonly<TunnelDto>[];
	settings: Readonly<SettingsDto>;
	host: string;
};

/** The local port as typed, or the remote port while it was not edited. */
export function getLocalPortText(
	entry: Readonly<DevslotPortEntry>,
	localPorts: DevslotLocalPorts
): string {
	return (
		localPorts[entry.key] ??
		String(entry.port.suggestedLocalPort ?? entry.port.port)
	);
}

function buildCandidate(
	entry: Readonly<DevslotPortEntry>,
	listenPort: number,
	host: string
) {
	return {
		host,
		kind: 'local',
		listenPort,
		listenAddress: DEVSLOT_LISTEN_ADDRESS,
		targetPort: entry.port.port,
	} as const;
}

function isBusyWithoutAlternative(
	entry: Readonly<DevslotPortEntry>,
	listenPort: number
): boolean {
	const { port } = entry;
	return (
		port.localPortBusy === true &&
		port.suggestedLocalPort === undefined &&
		listenPort === port.port
	);
}

/**
 * For each port, what stops it from being started as is: an invalid local port,
 * or a clash with an existing tunnel or with a port chosen before it. Only the
 * chosen ports clash with each other.
 */
export function findDevslotProblems({
	entries,
	localPorts,
	selectedKeys,
	tunnels,
	settings,
	host,
}: Readonly<FindDevslotProblemsInput>): Map<string, DevslotPortProblem | null> {
	const problems = new Map<string, DevslotPortProblem | null>();
	const parsed = entries.filter(isPortTunnelable).map((entry) => ({
		entry,
		listenPort: parsePort(getLocalPortText(entry, localPorts)),
	}));
	for (const { entry, listenPort } of parsed) {
		if (listenPort === null)
			problems.set(entry.key, { kind: 'invalidLocalPort' });
	}
	const valid = parsed.flatMap(({ entry, listenPort }) =>
		listenPort === null ? [] : [{ entry, listenPort }]
	);
	const chosen = valid.filter(({ entry }) => selectedKeys.has(entry.key));
	for (const { entry, listenPort } of valid) {
		if (isBusyWithoutAlternative(entry, listenPort)) {
			problems.set(entry.key, { kind: 'localPortBusy', port: listenPort });
		}
	}
	const chosenConflicts = findImportConflicts(
		chosen.map(({ entry, listenPort }) =>
			buildCandidate(entry, listenPort, host)
		),
		tunnels,
		settings,
		host
	);
	for (const [position, { entry }] of chosen.entries()) {
		if (problems.has(entry.key)) continue;
		problems.set(entry.key, chosenConflicts[position] ?? null);
	}
	for (const { entry, listenPort } of valid) {
		if (selectedKeys.has(entry.key) || problems.has(entry.key)) continue;
		const [conflict] = findImportConflicts(
			[buildCandidate(entry, listenPort, host)],
			tunnels,
			settings,
			host
		);
		problems.set(entry.key, conflict ?? null);
	}
	return problems;
}
