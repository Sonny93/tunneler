import type { TunnelDto } from '#/daemon/protocol';
import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';
import { getFailureReasonLabel } from '#/tunnels/failure_reason_label';

const MAXIMUM_DETAILED_CHANGES = 3;

function describeChange(tunnel: Readonly<TunnelDto>): string {
	const stateLabel = getTunnelStateLabel(tunnel.state);
	if (tunnel.state.kind !== 'failed') return `${tunnel.name} : ${stateLabel}`;
	return `${tunnel.name} : ${stateLabel}, ${getFailureReasonLabel(tunnel.state.reason)}`;
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
