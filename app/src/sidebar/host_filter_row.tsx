import { useUiStore } from '#/state/ui_store';
import { FilterButton } from '#/sidebar/filter_button';
import type { HostSummary } from '#/sidebar/summarize_hosts';
import { useResolvedFilter } from '#/tunnels/use_resolved_filter';

type HostFilterRowProps = {
	summary: HostSummary;
};

export function HostFilterRow({ summary }: Readonly<HostFilterRowProps>) {
	const { host } = summary;
	const filter = useResolvedFilter();
	const setFilter = useUiStore((state) => state.setFilter);
	const isActive = filter.kind === 'host' && filter.host === host;

	const handleSelect = () => setFilter({ kind: 'host', host });

	return (
		<li className="flex">
			<FilterButton
				label={host}
				count={summary}
				isActive={isActive}
				onSelect={handleSelect}
			/>
		</li>
	);
}
