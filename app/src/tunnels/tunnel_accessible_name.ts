import type { TunnelDto } from '#/daemon/protocol';
import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

/** Example: `Tunnel 3333 vers nova:3333, actif`, or `Tunnel 3333 vers nova:3333, temporaire, actif`. */
export function getTunnelAccessibleName(
	tunnel: Readonly<
		Pick<
			TunnelDto,
			| 'kind'
			| 'host'
			| 'listenPort'
			| 'targetHost'
			| 'targetPort'
			| 'state'
			| 'isTemporary'
		>
	>
): string {
	const endpoint = getTunnelAccessibleEndpointLabel(tunnel);
	const stateLabel = getTunnelStateLabel(tunnel.state).toLowerCase();
	const temporaryLabel = tunnel.isTemporary ? 'temporaire, ' : '';
	return `Tunnel ${endpoint}, ${temporaryLabel}${stateLabel}`;
}
