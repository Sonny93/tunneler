import type { TunnelDto } from '#/daemon/protocol';

/** The connections currently open through `tunnels`. */
export function countOpenConnections(
	tunnels: readonly Readonly<TunnelDto>[]
): number {
	return tunnels.reduce(
		(total, { metrics }) => total + metrics.activeConnections,
		0
	);
}
