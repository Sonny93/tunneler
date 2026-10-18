import { useEffect, useId, useRef } from 'react';
import { ContextMenu, MenuItem, Switch } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { FilterButton } from '#/sidebar/filter_button';
import type { TagSummary } from '#/sidebar/summarize_tags';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useResolvedFilter } from '#/tunnels/use_resolved_filter';
import { describeActiveCount } from '#/tunnels/describe_active_count';

type TagFilterRowProps = {
	summary: TagSummary;
};

export function TagFilterRow({ summary }: Readonly<TagFilterRowProps>) {
	const { tag, running, total } = summary;
	const descriptionId = useId();
	const filter = useResolvedFilter();
	const setFilter = useUiStore((state) => state.setFilter);
	const rowRef = useRef<HTMLLIElement>(null);
	const pendingFocusTag = useUiStore((state) => state.pendingFocusTag);
	const openTagRename = useUiStore((state) => state.openTagRename);
	const runDaemonAction = useDaemonAction();
	const isEveryTunnelRunning = total > 0 && running === total;
	const isActive = filter.kind === 'tag' && filter.tag === tag;

	useEffect(() => {
		if (pendingFocusTag !== tag) return;
		rowRef.current?.querySelector('button')?.focus();
		useUiStore.getState().clearPendingFocusTag();
	}, [pendingFocusTag, tag]);

	const handleSelect = () => setFilter({ kind: 'tag', tag });

	const handleRename = () => openTagRename(tag, 'sidebar');

	const handleToggle = () => {
		if (isEveryTunnelRunning) {
			void runDaemonAction(
				(gateway) => gateway.stopTag(tag),
				'Arrêt impossible'
			);
			return;
		}
		void runDaemonAction(
			(gateway) => gateway.startTag(tag),
			'Démarrage impossible'
		);
	};

	const menuItems = (
		<>
			<MenuItem icon="i-mdi-pencil-outline" onClick={handleRename}>
				Renommer…
			</MenuItem>
			<MenuItem
				icon={isEveryTunnelRunning ? 'i-mdi-stop' : 'i-mdi-play'}
				onClick={handleToggle}
			>
				{isEveryTunnelRunning ? 'Arrêter le tag' : 'Démarrer le tag'}
			</MenuItem>
		</>
	);

	return (
		<li ref={rowRef}>
			<ContextMenu
				items={menuItems}
				aria-label={`Actions pour le tag ${tag}`}
				className="flex items-center gap-1"
			>
				<FilterButton
					label={tag}
					count={summary}
					isActive={isActive}
					onSelect={handleSelect}
				/>
				<Switch
					aria-label={`Tous les tunnels du tag ${tag}`}
					aria-describedby={descriptionId}
					checked={isEveryTunnelRunning}
					onChange={handleToggle}
					wrapperClassName="shrink-0"
				/>
				<span id={descriptionId} className="sr-only">
					{describeActiveCount(summary)}
				</span>
			</ContextMenu>
		</li>
	);
}
