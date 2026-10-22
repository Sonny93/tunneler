import { useLingui } from '@lingui/react/macro';
import { useEffect, useId, useRef } from 'react';
import { ContextMenu, MenuItem, Switch } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { FilterButton } from '#/sidebar/filter_button';
import type { TagSummary } from '#/sidebar/summarize_tags';
import { useDeleteTag } from '#/tag_delete/use_delete_tag';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useSendToStandby } from '#/standby/use_send_to_standby';
import { useResolvedFilter } from '#/tunnels/use_resolved_filter';
import { describeActiveCount } from '#/tunnels/describe_active_count';

type TagFilterRowProps = {
	summary: TagSummary;
};

export function TagFilterRow({ summary }: Readonly<TagFilterRowProps>) {
	const { t, i18n } = useLingui();
	const { tag, started, total } = summary;
	const descriptionId = useId();
	const filter = useResolvedFilter();
	const setFilter = useUiStore((state) => state.setFilter);
	const rowRef = useRef<HTMLLIElement>(null);
	const pendingFocusTag = useUiStore((state) => state.pendingFocusTag);
	const openTagRename = useUiStore((state) => state.openTagRename);
	const runDaemonAction = useDaemonAction();
	const deleteTag = useDeleteTag();
	const { sendTag } = useSendToStandby();
	const hasAwakeTunnel = useDaemonStore((state) =>
		state.tunnels.some((tunnel) => tunnel.isAwake && tunnel.tags.includes(tag))
	);
	const isEveryTunnelStarted = total > 0 && started === total;
	const isActive = filter.kind === 'tag' && filter.tag === tag;

	useEffect(() => {
		if (pendingFocusTag !== tag) return;
		rowRef.current?.querySelector('button')?.focus();
		useUiStore.getState().clearPendingFocusTag();
	}, [pendingFocusTag, tag]);

	const handleSelect = () => setFilter({ kind: 'tag', tag });

	const handleRename = () => openTagRename(tag, 'sidebar');

	const handleDelete = () => void deleteTag(tag);

	const handleStandby = () => void sendTag(tag);

	const handleToggle = () => {
		if (isEveryTunnelStarted) {
			void runDaemonAction(
				(gateway) => gateway.stopTag(tag),
				t`Couldn't stop the tunnels`
			);
			return;
		}
		void runDaemonAction(
			(gateway) => gateway.startTag(tag),
			t`Couldn't start the tunnels`
		);
	};

	const menuItems = (
		<>
			<MenuItem icon="i-mdi-pencil-outline" onClick={handleRename}>
				{t`Rename…`}
			</MenuItem>
			<MenuItem icon="i-mdi-tag-remove-outline" danger onClick={handleDelete}>
				{t`Delete tag…`}
			</MenuItem>
			<MenuItem
				icon={isEveryTunnelStarted ? 'i-mdi-stop' : 'i-mdi-play'}
				onClick={handleToggle}
			>
				{isEveryTunnelStarted ? t`Stop the tag` : t`Start the tag`}
			</MenuItem>
			<MenuItem
				icon="i-mdi-sleep"
				disabled={!hasAwakeTunnel}
				onClick={handleStandby}
			>
				{t`Send the tag to standby`}
			</MenuItem>
		</>
	);

	return (
		<li ref={rowRef}>
			<ContextMenu
				items={menuItems}
				aria-label={t`Actions for tag ${tag}`}
				className="flex items-center gap-1"
			>
				<FilterButton
					label={tag}
					count={summary}
					icon="i-mdi-tag-outline"
					isActive={isActive}
					onSelect={handleSelect}
				/>
				<Switch
					aria-label={t`All tunnels of tag ${tag}`}
					aria-describedby={descriptionId}
					checked={isEveryTunnelStarted}
					onChange={handleToggle}
					wrapperClassName="shrink-0"
				/>
				<span id={descriptionId} className="sr-only">
					{describeActiveCount(summary, i18n)}
				</span>
			</ContextMenu>
		</li>
	);
}
