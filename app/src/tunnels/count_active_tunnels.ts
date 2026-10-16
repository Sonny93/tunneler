import type { TunnelDto } from '#/daemon/protocol';
import type { ActiveCount } from '#/tunnels/active_count';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';

export function countActiveTunnels(
	tunnels: readonly Readonly<TunnelDto>[]
): ActiveCount {
	return {
		running: tunnels.filter((tunnel) => isTunnelRunning(tunnel.state)).length,
		total: tunnels.length,
	};
}
