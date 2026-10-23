import { useLingui } from '@lingui/react/macro';
import { ContextMenu, MenuItem } from '@minimalstuff/ui';
import { useEffect, useMemo, useRef } from 'react';

import { FilterButton } from '#/sidebar/filter_button';
import { HostFilterRow } from '#/sidebar/host_filter_row';
import { SIDEBAR_ID } from '#/sidebar/sidebar_id';
import { summarizeHosts } from '#/sidebar/summarize_hosts';
import { summarizeTags } from '#/sidebar/summarize_tags';
import { TagFilterRow } from '#/sidebar/tag_filter_row';
import { listAwakeTunnels } from '#/standby/list_awake_tunnels';
import { useSendToStandby } from '#/standby/use_send_to_standby';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { countActiveTunnels } from '#/tunnels/count_active_tunnels';
import { useResolvedFilter } from '#/tunnels/use_resolved_filter';

const SECTION_HEADING_CLASS_NAME =
	'mb-1 px-2 text-xs font-semibold text-tn-muted';

export function Sidebar() {
	const { t } = useLingui();
	const tunnels = useDaemonStore((state) => state.tunnels);
	const filter = useResolvedFilter();
	const setFilter = useUiStore((state) => state.setFilter);
	const tagSummaries = useMemo(() => summarizeTags(tunnels), [tunnels]);
	const hostSummaries = useMemo(() => summarizeHosts(tunnels), [tunnels]);

	const { sendAll } = useSendToStandby();
	const hasAwakeTunnel = listAwakeTunnels(tunnels).length > 0;

	const allFilterRef = useRef<HTMLDivElement>(null);
	const isAllFilterFocusPending = useUiStore(
		(state) => state.isAllFilterFocusPending
	);

	useEffect(() => {
		if (!isAllFilterFocusPending) return;
		allFilterRef.current?.querySelector('button')?.focus();
		useUiStore.getState().clearAllFilterFocus();
	}, [isAllFilterFocusPending]);

	const handleSelectAll = () => setFilter({ kind: 'all' });

	const handleStandbyAll = () => void sendAll();

	return (
		<nav
			id={SIDEBAR_ID}
			aria-label={t`Tunnel filters`}
			className="flex flex-col gap-4"
		>
			<div ref={allFilterRef} className="flex">
				<ContextMenu
					items={
						<MenuItem
							icon="i-mdi-sleep"
							disabled={!hasAwakeTunnel}
							onClick={handleStandbyAll}
						>
							{t`Send awake tunnels to standby`}
						</MenuItem>
					}
					aria-label={t`Actions for all tunnels`}
					className="flex flex-1"
				>
					<FilterButton
						label={t`All tunnels`}
						count={countActiveTunnels(tunnels)}
						icon="i-mdi-format-list-bulleted"
						isActive={filter.kind === 'all'}
						onSelect={handleSelectAll}
					/>
				</ContextMenu>
			</div>
			{hostSummaries.length > 0 && (
				<section>
					<h2 className={SECTION_HEADING_CLASS_NAME}>{t`Hosts`}</h2>
					<ul role="list" className="flex flex-col gap-0.5">
						{hostSummaries.map((summary) => (
							<HostFilterRow key={summary.host} summary={summary} />
						))}
					</ul>
				</section>
			)}
			{tagSummaries.length > 0 && (
				<section>
					<h2 className={SECTION_HEADING_CLASS_NAME}>{t`Tags`}</h2>
					<ul role="list" className="flex flex-col gap-0.5">
						{tagSummaries.map((summary) => (
							<TagFilterRow key={summary.tag} summary={summary} />
						))}
					</ul>
				</section>
			)}
		</nav>
	);
}
