import type { TunnelDto } from '#/daemon/protocol';
import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

/** Example: `Tunnel Adonis, 3333 vers nova:3333, actif`. */
export function getTunnelAccessibleName(
	tunnel: Readonly<
		Pick<
			TunnelDto,
			| 'name'
			| 'kind'
			| 'host'
			| 'listenPort'
			| 'targetHost'
			| 'targetPort'
			| 'state'
		>
	>
): string {
	const endpoint = getTunnelAccessibleEndpointLabel(tunnel);
	const stateLabel = getTunnelStateLabel(tunnel.state).toLowerCase();
	return `Tunnel ${tunnel.name}, ${endpoint}, ${stateLabel}`;
}
