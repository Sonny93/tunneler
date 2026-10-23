import type { I18n } from '@lingui/core';
import { plural, t } from '@lingui/core/macro';

import type { TunnelDto } from '#/daemon/protocol';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';
import { getTunnelStateDetail } from '#/tunnels/tunnel_state_detail';
import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';

const MAXIMUM_DETAILED_CHANGES = 3;

function describeChange(tunnel: Readonly<TunnelDto>, i18n: I18n): string {
	const label = getTunnelAccessibleEndpointLabel(tunnel, i18n);
	const stateLabel = getTunnelStateLabel(tunnel.state, i18n);
	const detail = getTunnelStateDetail(tunnel.state, i18n);
	if (detail === null) return t(i18n)`${label}: ${stateLabel}`;
	return t(i18n)`${label}: ${stateLabel}, ${detail}`;
}

/** One line for a single change; a count, plus the details when there are few, for several. */
export function buildStateAnnouncement(
	changedTunnels: readonly TunnelDto[],
	i18n: I18n
): string {
	const [firstTunnel] = changedTunnels;
	if (!firstTunnel) return '';
	if (changedTunnels.length === 1) return describeChange(firstTunnel, i18n);
	const changedCount = changedTunnels.length;
	const summary = t(i18n)`${plural(changedCount, {
		one: '# tunnel changed state',
		other: '# tunnels changed state',
	})}`;
	if (changedTunnels.length > MAXIMUM_DETAILED_CHANGES) return summary;
	const details = changedTunnels
		.map((tunnel) => describeChange(tunnel, i18n))
		.join('. ');
	return `${summary}. ${details}`;
}
