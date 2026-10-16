import type { TunnelDefinitionDto } from '#/daemon/protocol';

const LOOPBACK_HOSTS: readonly string[] = ['127.0.0.1', 'localhost', '::1'];
export const VISUAL_SEPARATOR = '→';
export const SPOKEN_SEPARATOR = 'vers';

type EndpointSource = Pick<
	TunnelDefinitionDto,
	'host' | 'listenPort' | 'targetHost' | 'targetPort'
>;

function buildEndpointLabel(
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

/** Local tunnel endpoint, such as `3333 → nova:3333`. */
export function getTunnelEndpointLabel(
	tunnel: Readonly<EndpointSource>
): string {
	return buildEndpointLabel(tunnel, VISUAL_SEPARATOR);
}

/** Same endpoint with "vers" instead of an arrow, which screen readers read inconsistently. */
export function getTunnelAccessibleEndpointLabel(
	tunnel: Readonly<EndpointSource>
): string {
	return buildEndpointLabel(tunnel, SPOKEN_SEPARATOR);
}
