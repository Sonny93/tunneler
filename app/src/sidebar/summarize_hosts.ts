import type { TunnelDto } from '#/daemon/protocol';
import type { ActiveCount } from '#/tunnels/active_count';
import { countActiveTunnels } from '#/tunnels/count_active_tunnels';
import { groupTunnelsByHost } from '#/tunnels/group_tunnels_by_host';

export type HostSummary = ActiveCount & { host: string };

/** One entry per host used by at least one tunnel, sorted case-insensitively. */
export function summarizeHosts(
	tunnels: readonly Readonly<TunnelDto>[]
): readonly HostSummary[] {
	return groupTunnelsByHost(tunnels).map(({ host, tunnels: hostTunnels }) => ({
		host,
		...countActiveTunnels(hostTunnels),
	}));
}
