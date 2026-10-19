import { IconButton } from '@minimalstuff/ui';
import { useId, useMemo, useRef } from 'react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { filterTunnels } from '#/tunnels/filter_tunnels';
import { TunnelGroup } from '#/tunnels/list/tunnel_group';
import { useDeleteTag } from '#/tag_delete/use_delete_tag';
import { useRovingRows } from '#/tunnels/list/use_roving_rows';
import { BulkToolbar } from '#/tunnels/selection/bulk_toolbar';
import { useResolvedFilter } from '#/tunnels/use_resolved_filter';
import { TunnelListEmpty } from '#/tunnels/list/tunnel_list_empty';
import { groupTunnelsByHost } from '#/tunnels/group_tunnels_by_host';
import { describeTunnelFilter } from '#/tunnels/describe_tunnel_filter';
import { RemovalFocusContext } from '#/tunnels/list/removal_focus_context';
import { SelectAllCheckbox } from '#/tunnels/selection/select_all_checkbox';
import { useTunnelSelection } from '#/tunnels/selection/use_tunnel_selection';
import { useFocusAfterRemoval } from '#/tunnels/list/use_focus_after_removal';
import { useFocusAfterCreation } from '#/tunnels/list/use_focus_after_creation';
import { useSelectionShortcuts } from '#/tunnels/selection/use_selection_shortcuts';
import { TunnelSelectionContext } from '#/tunnels/selection/tunnel_selection_context';
import { useAnnounceSelectionCount } from '#/tunnels/selection/use_announce_selection_count';

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
	const orderedTunnels = useMemo(
		() => groups.flatMap(({ tunnels: group }) => group),
		[groups]
	);
	const rowIds = useMemo(
		() => orderedTunnels.map(({ id }) => id),
		[orderedTunnels]
	);
	const selection = useTunnelSelection(orderedTunnels);
	const selectedCount = selection.selectedTunnels.length;
	const selectTunnels = useUiStore((state) => state.selectTunnels);
	const clearTunnelSelection = useUiStore(
		(state) => state.clearTunnelSelection
	);
	const openTagRename = useUiStore((state) => state.openTagRename);
	const deleteTag = useDeleteTag();
	const createButtonRef = useRef<HTMLButtonElement>(null);
	const { activeRowId, containerProps } = useRovingRows(rowIds);
	const requestFocusAfterRemoval = useFocusAfterRemoval(
		rowIds,
		containerProps.ref,
		() => createButtonRef.current?.focus()
	);

	useFocusAfterCreation(rowIds, containerProps.ref);
	useAnnounceSelectionCount(selectedCount);

	const handleRenameTag = () => {
		if (filter.kind === 'tag') openTagRename(filter.tag, 'list');
	};
	const handleDeleteTag = () => {
		if (filter.kind === 'tag') void deleteTag(filter.tag);
	};
	const handleSelectAll = () => selectTunnels(rowIds);
	const handleClearSelection = () => {
		if (selectedCount > 0) clearTunnelSelection();
	};
	const handleSelectionKeyDown = useSelectionShortcuts({
		onSelectAll: handleSelectAll,
		onClear: handleClearSelection,
	});

	return (
		<div className="flex flex-col gap-4">
			<div className="flex flex-wrap items-center gap-2">
				<h2 className="text-base font-semibold">
					{describeTunnelFilter(filter)} ({visibleTunnels.length})
				</h2>
				{filter.kind === 'tag' && (
					<IconButton
						icon="i-mdi-pencil-outline"
						aria-label="Renommer le tag"
						tooltip
						variant="ghost"
						color="neutral"
						onClick={handleRenameTag}
					/>
				)}
				{filter.kind === 'tag' && (
					<IconButton
						icon="i-mdi-tag-remove-outline"
						aria-label="Supprimer le tag"
						tooltip
						variant="ghost"
						color="neutral"
						onClick={handleDeleteTag}
					/>
				)}
			</div>
			{visibleTunnels.length === 0 ? (
				<TunnelListEmpty createButtonRef={createButtonRef} />
			) : (
				<>
					<p id={hintId} className="sr-only">
						Utilisez les flèches haut et bas pour passer d’un tunnel à l’autre.
					</p>
					<RemovalFocusContext value={requestFocusAfterRemoval}>
						<TunnelSelectionContext value={selection}>
							{/* oxlint-disable-next-line jsx-a11y/no-static-element-interactions -- only Ctrl+A and Escape bubbling up from the controls inside are handled */}
							<div
								onKeyDown={handleSelectionKeyDown}
								className="flex flex-col gap-4"
							>
								<div className="flex flex-wrap items-center gap-x-6 gap-y-2">
									<SelectAllCheckbox
										visibleCount={rowIds.length}
										selectedCount={selectedCount}
										onSelectAll={handleSelectAll}
										onClear={handleClearSelection}
									/>
									{selectedCount > 0 && (
										<BulkToolbar selectedTunnels={selection.selectedTunnels} />
									)}
								</div>
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
							</div>
						</TunnelSelectionContext>
					</RemovalFocusContext>
				</>
			)}
		</div>
	);
}
