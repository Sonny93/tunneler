import type { TunnelDto } from '#/daemon/protocol';
import type { ActiveCount } from '#/tunnels/active_count';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { countActiveTunnels } from '#/tunnels/count_active_tunnels';
import { compareTextCaseInsensitively } from '#/tunnels/compare_text_case_insensitively';

/** `running` counts active tunnels only; `started` also counts those in standby. */
export type TagSummary = ActiveCount & { tag: string; started: number };

const isStarted = (tunnel: Readonly<TunnelDto>) =>
	isTunnelRunning(tunnel.state);

/** One entry per tag in use, sorted case-insensitively. */
export function summarizeTags(
	tunnels: readonly Readonly<TunnelDto>[]
): readonly TagSummary[] {
	const tunnelsByTag = new Map<string, TunnelDto[]>();
	for (const tunnel of tunnels) {
		for (const tag of new Set(tunnel.tags)) {
			const tagTunnels = tunnelsByTag.get(tag) ?? [];
			tagTunnels.push(tunnel);
			tunnelsByTag.set(tag, tagTunnels);
		}
	}

	return [...tunnelsByTag]
		.map(([tag, tagTunnels]) => ({
			tag,
			...countActiveTunnels(tagTunnels),
			started: tagTunnels.filter(isStarted).length,
		}))
		.sort((left, right) => compareTextCaseInsensitively(left.tag, right.tag));
}
