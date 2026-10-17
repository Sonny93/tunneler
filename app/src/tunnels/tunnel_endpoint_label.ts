import type { TunnelDefinitionDto } from '#/daemon/protocol';

const LOOPBACK_HOSTS: readonly string[] = ['127.0.0.1', 'localhost', '::1'];
const DEFAULT_REMOTE_TARGET_HOST = '127.0.0.1';
export const VISUAL_SEPARATOR = '→';
export const SPOKEN_SEPARATOR = 'vers';

type EndpointSource = Pick<
	TunnelDefinitionDto,
	'kind' | 'host' | 'listenPort' | 'targetHost' | 'targetPort'
>;

function buildLocalLabel(
	tunnel: Readonly<EndpointSource>,
	separator: string
): string {
	const targetPort = tunnel.targetPort ?? tunnel.listenPort;
	const { targetHost } = tunnel;
	const isTargetOnSshHost = !targetHost || LOOPBACK_HOSTS.includes(targetHost);
	if (isTargetOnSshHost) {
		return `${tunnel.listenPort} ${separator} ${tunnel.host}:${targetPort}`;
	}
	return `${tunnel.listenPort} ${separator} ${targetHost}:${targetPort} via ${tunnel.host}`;
}

function buildRemoteTarget(tunnel: Readonly<EndpointSource>): string {
	const targetHost = tunnel.targetHost ?? DEFAULT_REMOTE_TARGET_HOST;
	return `${targetHost}:${tunnel.targetPort ?? tunnel.listenPort}`;
}

/**
 * Example: `3333 → nova:3333` for a local forward, `nova:8080 → 127.0.0.1:3000`
 * for a remote one, `1080 (SOCKS via nova)` for a dynamic one.
 */
export function getTunnelEndpointLabel(
	tunnel: Readonly<EndpointSource>
): string {
	switch (tunnel.kind) {
		case 'local':
			return buildLocalLabel(tunnel, VISUAL_SEPARATOR);
		case 'remote':
			return `${tunnel.host}:${tunnel.listenPort} ${VISUAL_SEPARATOR} ${buildRemoteTarget(tunnel)}`;
		case 'dynamic':
			return `${tunnel.listenPort} (SOCKS via ${tunnel.host})`;
	}
}

/** Same endpoint in words, which screen readers read consistently (no arrow). */
export function getTunnelAccessibleEndpointLabel(
	tunnel: Readonly<EndpointSource>
): string {
	switch (tunnel.kind) {
		case 'local':
			return buildLocalLabel(tunnel, SPOKEN_SEPARATOR);
		case 'remote':
			return `port ${tunnel.listenPort} du serveur ${tunnel.host} ${SPOKEN_SEPARATOR} ${buildRemoteTarget(tunnel)}`;
		case 'dynamic':
			return `proxy SOCKS sur le port ${tunnel.listenPort} via ${tunnel.host}`;
	}
}
