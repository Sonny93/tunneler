import type { TunnelFilter } from '#/state/ui_store';

export function describeTunnelFilter(filter: Readonly<TunnelFilter>): string {
	switch (filter.kind) {
		case 'all':
			return 'Tous les tunnels';
		case 'tag':
			return `Tag ${filter.tag}`;
		case 'host':
			return `Host ${filter.host}`;
	}
}
