import type { TunnelDto } from '#/daemon/protocol';
import type { ActiveCount } from '#/tunnels/active_count';
import { countActiveTunnels } from '#/tunnels/count_active_tunnels';
import { compareTextCaseInsensitively } from '#/tunnels/compare_text_case_insensitively';

export type TagSummary = ActiveCount & { tag: string };

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
		.map(([tag, tagTunnels]) => ({ tag, ...countActiveTunnels(tagTunnels) }))
		.sort((left, right) => compareTextCaseInsensitively(left.tag, right.tag));
}
