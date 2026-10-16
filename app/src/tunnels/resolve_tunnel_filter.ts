import type { TunnelDto } from '#/daemon/protocol';
import type { TunnelFilter } from '#/state/ui_store';

const SHOW_ALL: TunnelFilter = { kind: 'all' };

/** Falls back to showing everything when the filtered tag or host no longer exists. */
export function resolveTunnelFilter(
	tunnels: readonly Readonly<TunnelDto>[],
	filter: Readonly<TunnelFilter>
): TunnelFilter {
	switch (filter.kind) {
		case 'all':
			return filter;
		case 'tag':
			return tunnels.some((tunnel) => tunnel.tags.includes(filter.tag))
				? filter
				: SHOW_ALL;
		case 'host':
			return tunnels.some((tunnel) => tunnel.host === filter.host)
				? filter
				: SHOW_ALL;
	}
}
