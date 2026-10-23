import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type { TunnelDto } from '#/daemon/protocol';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';
import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';

/** Example: `Tunnel 3333 to nova:3333, active`, or `Tunnel 3333 to nova:3333, temporary, active`. */
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
	>,
	i18n: I18n
): string {
	const endpoint = getTunnelAccessibleEndpointLabel(tunnel, i18n);
	const stateLabel = getTunnelStateLabel(tunnel.state, i18n).toLowerCase();
	if (tunnel.isTemporary) {
		return t(i18n)`Tunnel ${endpoint}, temporary, ${stateLabel}`;
	}
	return t(i18n)`Tunnel ${endpoint}, ${stateLabel}`;
}
