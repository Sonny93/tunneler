import { useEffect, useMemo } from 'react';

import type { TunnelDto } from '#/daemon/protocol';
import { useUiStore } from '#/state/ui_store';
import type { TunnelSelection } from '#/tunnels/selection/tunnel_selection_context';

/**
 * The selection restricted to `orderedTunnels` (the visible tunnels in list
 * order). Ids that left the list (filter change, deletion) are also pruned
 * from the store so they do not come back with the filter.
 */
export function useTunnelSelection(
	orderedTunnels: readonly TunnelDto[]
): TunnelSelection {
	const storedIds = useUiStore((state) => state.selectedTunnelIds);
	const toggleTunnelSelection = useUiStore(
		(state) => state.toggleTunnelSelection
	);
	const selectTunnelRange = useUiStore((state) => state.selectTunnelRange);
	const retainTunnelSelection = useUiStore(
		(state) => state.retainTunnelSelection
	);
	const orderedIds = useMemo(
		() => orderedTunnels.map(({ id }) => id),
		[orderedTunnels]
	);
	const selectedTunnels = useMemo(
		() => orderedTunnels.filter(({ id }) => storedIds.has(id)),
		[orderedTunnels, storedIds]
	);
	const selectedIds = useMemo(
		() => new Set(selectedTunnels.map(({ id }) => id)),
		[selectedTunnels]
	);

	useEffect(() => {
		if (selectedIds.size !== storedIds.size) retainTunnelSelection(orderedIds);
	}, [selectedIds, storedIds, orderedIds, retainTunnelSelection]);

	return {
		selectedIds,
		selectedTunnels,
		toggle: toggleTunnelSelection,
		selectRange: (tunnelId) => selectTunnelRange(tunnelId, orderedIds),
	};
}
