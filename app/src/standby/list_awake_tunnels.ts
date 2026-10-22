import type { TunnelDto } from '#/daemon/protocol';

/** The tunnels woken from standby, which can be sent back to it. */
export function listAwakeTunnels(
	tunnels: readonly Readonly<TunnelDto>[]
): TunnelDto[] {
	return tunnels.filter(({ isAwake }) => isAwake);
}
