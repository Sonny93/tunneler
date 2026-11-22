import type { ImportConflictDto, ImportRouteDto } from '#/daemon/protocol';
import {
	isPortTunnelable,
	type DevslotPortEntry,
} from '#/import/devslot/devslot_ports';
import { parsePort } from '#/tunnel_form/parse_port';

export const DEVSLOT_LISTEN_ADDRESS = '127.0.0.1';

export type DevslotPortProblem =
	| { kind: 'invalidLocalPort' }
	| { kind: 'localPortBusy'; port: number }
	| ImportConflictDto;

export type DevslotLocalPorts = Readonly<Record<string, string>>;

type ProblemsInput = {
	entries: readonly DevslotPortEntry[];
	localPorts: DevslotLocalPorts;
};

type ClassifyRequestInput = ProblemsInput & {
	selectedKeys: ReadonlySet<string>;
	host: string;
};

/** What to ask the daemon: the routes in order, the port each one stands for, and which ports are chosen. */
export type DevslotClassifyRequest = {
	routes: ImportRouteDto[];
	keys: string[];
	chosenKeys: string[];
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

function buildRoute(
	entry: Readonly<DevslotPortEntry>,
	listenPort: number,
	host: string
): ImportRouteDto {
	return {
		host,
		kind: 'local',
		listenPort,
		listenAddress: DEVSLOT_LISTEN_ADDRESS,
		targetPort: entry.port.port,
	};
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

function parseLocalPorts({ entries, localPorts }: Readonly<ProblemsInput>) {
	return entries.filter(isPortTunnelable).map((entry) => ({
		entry,
		listenPort: parsePort(getLocalPortText(entry, localPorts)),
	}));
}

/** The problems a port has on its own, before any tunnel is considered: an unreadable or busy local port. */
export function findLocalPortProblems(
	input: Readonly<ProblemsInput>
): Map<string, DevslotPortProblem> {
	const problems = new Map<string, DevslotPortProblem>();
	for (const { entry, listenPort } of parseLocalPorts(input)) {
		if (listenPort === null) {
			problems.set(entry.key, { kind: 'invalidLocalPort' });
		} else if (isBusyWithoutAlternative(entry, listenPort)) {
			problems.set(entry.key, { kind: 'localPortBusy', port: listenPort });
		}
	}
	return problems;
}

/**
 * Every port with a readable local port becomes a route, the chosen ports
 * first (so the daemon counts a port as duplicate only against another chosen
 * port, or against an earlier unchosen one), then the others.
 */
export function buildDevslotClassifyRequest({
	selectedKeys,
	host,
	...input
}: Readonly<ClassifyRequestInput>): DevslotClassifyRequest {
	const readable = parseLocalPorts(input).flatMap(({ entry, listenPort }) =>
		listenPort === null ? [] : [{ entry, listenPort }]
	);
	const chosen = readable.filter(({ entry }) => selectedKeys.has(entry.key));
	const others = readable.filter(({ entry }) => !selectedKeys.has(entry.key));
	const ordered = [...chosen, ...others];
	return {
		routes: ordered.map(({ entry, listenPort }) =>
			buildRoute(entry, listenPort, host)
		),
		keys: ordered.map(({ entry }) => entry.key),
		chosenKeys: chosen.map(({ entry }) => entry.key),
	};
}

/**
 * For each port, what stops it from being started as is: an unreadable or
 * busy local port, or the conflict the daemon last reported for its route.
 */
export function findDevslotProblems(
	input: Readonly<ProblemsInput>,
	request: Readonly<DevslotClassifyRequest>,
	conflictsByKey: ReadonlyMap<string, ImportConflictDto | null>
): Map<string, DevslotPortProblem | null> {
	const problems = new Map<string, DevslotPortProblem | null>(
		findLocalPortProblems(input)
	);
	for (const key of request.keys) {
		if (problems.has(key)) continue;
		problems.set(key, conflictsByKey.get(key) ?? null);
	}
	return problems;
}
