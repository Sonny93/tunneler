import { useId, useMemo, useRef } from 'react';

import { useDaemonStore } from '#/state/daemon_store';
import { filterTunnels } from '#/tunnels/filter_tunnels';
import { TunnelGroup } from '#/tunnels/list/tunnel_group';
import { useRovingRows } from '#/tunnels/list/use_roving_rows';
import { useResolvedFilter } from '#/tunnels/use_resolved_filter';
import { TunnelListEmpty } from '#/tunnels/list/tunnel_list_empty';
import { groupTunnelsByHost } from '#/tunnels/group_tunnels_by_host';
import { describeTunnelFilter } from '#/tunnels/describe_tunnel_filter';
import { RemovalFocusContext } from '#/tunnels/list/removal_focus_context';
import { useFocusAfterRemoval } from '#/tunnels/list/use_focus_after_removal';

export function TunnelList() {
	const tunnels = useDaemonStore((state) => state.tunnels);
	const filter = useResolvedFilter();
	const hintId = useId();
	const visibleTunnels = useMemo(
		() => filterTunnels(tunnels, filter),
		[tunnels, filter]
	);
	const groups = useMemo(
		() => groupTunnelsByHost(visibleTunnels),
		[visibleTunnels]
	);
	const rowIds = useMemo(
		() => groups.flatMap(({ tunnels: group }) => group.map(({ id }) => id)),
		[groups]
	);
	const createButtonRef = useRef<HTMLButtonElement>(null);
	const { activeRowId, containerProps } = useRovingRows(rowIds);
	const requestFocusAfterRemoval = useFocusAfterRemoval(
		rowIds,
		containerProps.ref,
		() => createButtonRef.current?.focus()
	);

	return (
		<div className="flex flex-col gap-4">
			<h2 className="text-base font-semibold">
				{describeTunnelFilter(filter)} ({visibleTunnels.length})
			</h2>
			{visibleTunnels.length === 0 ? (
				<TunnelListEmpty createButtonRef={createButtonRef} />
			) : (
				<>
					<p id={hintId} className="sr-only">
						Utilisez les flèches haut et bas pour passer d’un tunnel à l’autre.
					</p>
					<RemovalFocusContext value={requestFocusAfterRemoval}>
						<div {...containerProps} className="flex flex-col gap-4">
							{groups.map((group) => (
								<TunnelGroup
									key={group.host}
									group={group}
									activeRowId={activeRowId}
									hintId={hintId}
								/>
							))}
						</div>
					</RemovalFocusContext>
				</>
			)}
		</div>
	);
}
