import type { TunnelDto } from '#/daemon/protocol';
import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';
import { getTunnelStateDetail } from '#/tunnels/tunnel_state_detail';

const MAXIMUM_DETAILED_CHANGES = 3;

function describeChange(tunnel: Readonly<TunnelDto>): string {
	const stateLabel = getTunnelStateLabel(tunnel.state);
	const detail = getTunnelStateDetail(tunnel.state);
	if (detail === null) return `${tunnel.name} : ${stateLabel}`;
	return `${tunnel.name} : ${stateLabel}, ${detail}`;
}

/** One line for a single change; a count, plus the details when there are few, for several. */
export function buildStateAnnouncement(
	changedTunnels: readonly TunnelDto[]
): string {
	const [firstTunnel] = changedTunnels;
	if (!firstTunnel) return '';
	if (changedTunnels.length === 1) return describeChange(firstTunnel);
	const summary = `${changedTunnels.length} tunnels ont changé d'état`;
	if (changedTunnels.length > MAXIMUM_DETAILED_CHANGES) return summary;
	return `${summary}. ${changedTunnels.map(describeChange).join('. ')}`;
}
