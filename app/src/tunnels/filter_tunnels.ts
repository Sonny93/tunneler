import type { TunnelDto } from '#/daemon/protocol';
import type { TunnelFilter } from '#/state/ui_store';

export function filterTunnels(
	tunnels: readonly Readonly<TunnelDto>[],
	filter: Readonly<TunnelFilter>
): readonly TunnelDto[] {
	switch (filter.kind) {
		case 'all':
			return tunnels;
		case 'tag':
			return tunnels.filter((tunnel) => tunnel.tags.includes(filter.tag));
		case 'host':
			return tunnels.filter((tunnel) => tunnel.host === filter.host);
	}
}
