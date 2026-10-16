import { useMemo } from 'react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { FilterButton } from '#/sidebar/filter_button';
import { TagFilterRow } from '#/sidebar/tag_filter_row';
import { summarizeTags } from '#/sidebar/summarize_tags';
import { HostFilterRow } from '#/sidebar/host_filter_row';
import { summarizeHosts } from '#/sidebar/summarize_hosts';
import { useResolvedFilter } from '#/tunnels/use_resolved_filter';
import { countActiveTunnels } from '#/tunnels/count_active_tunnels';

const SECTION_HEADING_CLASS_NAME =
	'mb-1 px-2 text-xs font-semibold text-tn-muted';

export function Sidebar() {
	const tunnels = useDaemonStore((state) => state.tunnels);
	const filter = useResolvedFilter();
	const setFilter = useUiStore((state) => state.setFilter);
	const tagSummaries = useMemo(() => summarizeTags(tunnels), [tunnels]);
	const hostSummaries = useMemo(() => summarizeHosts(tunnels), [tunnels]);

	const handleSelectAll = () => setFilter({ kind: 'all' });

	return (
		<nav aria-label="Filtres des tunnels" className="flex flex-col gap-4">
			<div className="flex">
				<FilterButton
					label="Tous les tunnels"
					count={countActiveTunnels(tunnels)}
					isActive={filter.kind === 'all'}
					onSelect={handleSelectAll}
				/>
			</div>
			{tagSummaries.length > 0 && (
				<section>
					<h2 className={SECTION_HEADING_CLASS_NAME}>Tags</h2>
					<ul role="list" className="flex flex-col gap-0.5">
						{tagSummaries.map((summary) => (
							<TagFilterRow key={summary.tag} summary={summary} />
						))}
					</ul>
				</section>
			)}
			{hostSummaries.length > 0 && (
				<section>
					<h2 className={SECTION_HEADING_CLASS_NAME}>Hosts</h2>
					<ul role="list" className="flex flex-col gap-0.5">
						{hostSummaries.map((summary) => (
							<HostFilterRow key={summary.host} summary={summary} />
						))}
					</ul>
				</section>
			)}
		</nav>
	);
}
