import type { TunnelDto } from '#/daemon/protocol';
import { compareTextCaseInsensitively } from '#/tunnels/compare_text_case_insensitively';

export type TunnelGroup = {
	host: string;
	tunnels: readonly TunnelDto[];
};

/** Hosts sorted case-insensitively, and the tunnels of each host sorted by listen port. */
export function groupTunnelsByHost(
	tunnels: readonly Readonly<TunnelDto>[]
): readonly TunnelGroup[] {
	const tunnelsByHost = new Map<string, TunnelDto[]>();
	for (const tunnel of tunnels) {
		const hostTunnels = tunnelsByHost.get(tunnel.host) ?? [];
		hostTunnels.push(tunnel);
		tunnelsByHost.set(tunnel.host, hostTunnels);
	}

	return [...tunnelsByHost]
		.map(([host, hostTunnels]) => ({
			host,
			tunnels: [...hostTunnels].sort(
				(left, right) => left.listenPort - right.listenPort
			),
		}))
		.sort((left, right) => compareTextCaseInsensitively(left.host, right.host));
}
