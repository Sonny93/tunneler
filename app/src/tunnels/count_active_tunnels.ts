import type { TunnelDto } from '#/daemon/protocol';
import type { ActiveCount } from '#/tunnels/active_count';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';

/** A tunnel in standby holds no SSH session: it is not counted as active, nor shown as an error. */
function isTunnelActive(tunnel: Readonly<TunnelDto>): boolean {
	return isTunnelRunning(tunnel.state) && tunnel.state.kind !== 'standby';
}

export function countActiveTunnels(
	tunnels: readonly Readonly<TunnelDto>[]
): ActiveCount {
	return {
		running: tunnels.filter(isTunnelActive).length,
		total: tunnels.length,
	};
}
