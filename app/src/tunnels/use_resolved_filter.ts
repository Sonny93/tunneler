import { useMemo } from 'react';

import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore, type TunnelFilter } from '#/state/ui_store';
import { resolveTunnelFilter } from '#/tunnels/resolve_tunnel_filter';

/** The active filter, or "all" when its tag or host disappeared. */
export function useResolvedFilter(): TunnelFilter {
	const tunnels = useDaemonStore((state) => state.tunnels);
	const filter = useUiStore((state) => state.filter);

	return useMemo(() => resolveTunnelFilter(tunnels, filter), [tunnels, filter]);
}
