import type {
	ImportCandidateDto,
	SettingsDto,
	TunnelDto,
	TunnelKindDto,
} from '#/daemon/protocol';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

const DEFAULT_TARGET_HOST = '127.0.0.1';

export type ImportConflict =
	| { kind: 'alreadyConfigured' }
	| { kind: 'listenConflict'; tunnelLabel: string }
	| { kind: 'duplicateInImport' };

type ListenSettings = Pick<
	SettingsDto,
	'defaultBindAddress' | 'defaultRemoteBindAddress'
>;

type ListenIdentity = {
	host: string | null;
	kind: TunnelKindDto;
	address: string;
	port: number;
};

type ForwardSource = Pick<
	ImportCandidateDto,
	'kind' | 'listenPort' | 'listenAddress' | 'targetHost' | 'targetPort'
>;

function getListenAddress(
	forward: Readonly<ForwardSource>,
	settings: Readonly<ListenSettings>
): string {
	if (forward.listenAddress !== undefined) return forward.listenAddress;
	return forward.kind === 'remote'
		? settings.defaultRemoteBindAddress
		: settings.defaultBindAddress;
}

function buildListenIdentity(
	forward: Readonly<ForwardSource>,
	host: string | null,
	settings: Readonly<ListenSettings>
): ListenIdentity {
	return {
		host,
		kind: forward.kind,
		address: getListenAddress(forward, settings),
		port: forward.listenPort,
	};
}

function getTargetKey(forward: Readonly<ForwardSource>): string {
	if (forward.kind === 'dynamic') return '';
	const targetHost = forward.targetHost ?? DEFAULT_TARGET_HOST;
	return `${targetHost}:${forward.targetPort ?? forward.listenPort}`;
}

function isRemote(identity: Readonly<ListenIdentity>): boolean {
	return identity.kind === 'remote';
}

/** Local and dynamic forwards listen on this machine, remote ones on their ssh host. */
function areListenSidesInConflict(
	left: Readonly<ListenIdentity>,
	right: Readonly<ListenIdentity>
): boolean {
	if (isRemote(left) !== isRemote(right)) return false;
	if (left.address !== right.address || left.port !== right.port) return false;
	return !isRemote(left) || left.host === right.host;
}

function findAlreadyConfigured(
	candidate: Readonly<ImportCandidateDto>,
	host: string | null,
	existingTunnels: readonly Readonly<TunnelDto>[],
	settings: Readonly<ListenSettings>
): ImportConflict | null {
	if (host === null) return null;
	const candidateIdentity = buildListenIdentity(candidate, host, settings);
	const candidateTargetKey = getTargetKey(candidate);
	const match = existingTunnels.find(
		(tunnel) =>
			tunnel.host === host &&
			tunnel.kind === candidate.kind &&
			tunnel.listenPort === candidate.listenPort &&
			getListenAddress(tunnel, settings) === candidateIdentity.address &&
			getTargetKey(tunnel) === candidateTargetKey
	);
	return match ? { kind: 'alreadyConfigured' } : null;
}

function findListenConflict(
	identity: Readonly<ListenIdentity>,
	existingTunnels: readonly Readonly<TunnelDto>[],
	settings: Readonly<ListenSettings>
): ImportConflict | null {
	const match = existingTunnels.find((tunnel) =>
		areListenSidesInConflict(
			identity,
			buildListenIdentity(tunnel, tunnel.host, settings)
		)
	);
	return match
		? { kind: 'listenConflict', tunnelLabel: getTunnelEndpointLabel(match) }
		: null;
}

/**
 * For each candidate, what stops it from being imported as is: an identical
 * existing tunnel, an existing tunnel listening on the same address and port,
 * or an earlier candidate doing so. `fallbackHost` stands in for candidates
 * without a host.
 */
export function findImportConflicts(
	candidates: readonly Readonly<ImportCandidateDto>[],
	existingTunnels: readonly Readonly<TunnelDto>[],
	settings: Readonly<ListenSettings>,
	fallbackHost: string | null
): (ImportConflict | null)[] {
	const earlierIdentities: ListenIdentity[] = [];
	return candidates.map((candidate) => {
		const host = candidate.host ?? fallbackHost;
		const identity = buildListenIdentity(candidate, host, settings);
		const conflict =
			findAlreadyConfigured(candidate, host, existingTunnels, settings) ??
			findListenConflict(identity, existingTunnels, settings) ??
			(earlierIdentities.some((earlier) =>
				areListenSidesInConflict(earlier, identity)
			)
				? ({ kind: 'duplicateInImport' } as const)
				: null);
		earlierIdentities.push(identity);
		return conflict;
	});
}
