import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import type { TunnelDto, X11AnchorDto } from '#/daemon/protocol';

const RUNNING_X11_STATE_KINDS: readonly X11AnchorDto['state']['kind'][] = [
	'starting',
	'active',
	'reconnecting',
];

/** Tunnels and X11 anchors that hold resources, which a daemon quit would stop. */
export function countRunningSessions(
	tunnels: readonly TunnelDto[],
	x11Anchors: readonly X11AnchorDto[]
): number {
	const runningTunnels = tunnels.filter((tunnel) =>
		isTunnelRunning(tunnel.state)
	);
	const runningAnchors = x11Anchors.filter((anchor) =>
		RUNNING_X11_STATE_KINDS.includes(anchor.state.kind)
	);
	return runningTunnels.length + runningAnchors.length;
}
