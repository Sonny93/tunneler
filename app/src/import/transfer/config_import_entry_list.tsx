import { useLingui } from '@lingui/react/macro';
import { Button, Checkbox } from '@minimalstuff/ui';

import type { ConfigImportEntryDto } from '#/daemon/protocol';
import { describeSelectionCount } from '#/import/import_texts';
import { ConfigImportTextView } from '#/import/transfer/config_import_text';
import { describeConfigImportEntry } from '#/import/transfer/describe_config_import';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useDaemonStore } from '#/state/daemon_store';
import { getTunnelAccessibleEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

type ConfigImportEntryListProps = {
	entries: readonly ConfigImportEntryDto[];
	selectedIndexes: ReadonlySet<number>;
	onToggle: (index: number) => void;
	onSelectAll: () => void;
	onSelectNone: () => void;
};

/** The entries in file order, numbered from 1: the numbers are the positions the reasons refer to. */
export function ConfigImportEntryList({
	entries,
	selectedIndexes,
	onToggle,
	onSelectAll,
	onSelectNone,
}: Readonly<ConfigImportEntryListProps>) {
	const { t, i18n } = useLingui();
	const tunnels = useDaemonStore((state) => state.tunnels);
	const entryCount = entries.length;
	const importable = entries.filter(
		({ status }) => status.kind === 'importable'
	);
	const riskyCount = importable.filter(({ risks }) => risks.length > 0).length;

	const announce = (count: number, risky: number) =>
		useAnnouncerStore
			.getState()
			.announce(describeSelectionCount(count, risky, i18n), 'polite');
	const handleSelectAll = () => {
		onSelectAll();
		announce(importable.length, riskyCount);
	};
	const handleSelectNone = () => {
		onSelectNone();
		announce(0, 0);
	};

	return (
		<fieldset className="m-0 flex flex-col gap-3 border-0 p-0">
			<legend className="mb-2 text-sm font-semibold">
				{t`Tunnels in the file (${entryCount})`}
			</legend>
			<div className="flex flex-wrap gap-2">
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					onClick={handleSelectAll}
				>
					{t`Select all`}
				</Button>
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					onClick={handleSelectNone}
				>
					{t`Deselect all`}
				</Button>
			</div>
			<ol className="m-0 flex list-decimal flex-col gap-3 pl-8">
				{entries.map((entry) => {
					const label = getTunnelAccessibleEndpointLabel(entry.tunnel, i18n);
					const details = describeConfigImportEntry(
						entry,
						{ tunnels, entries },
						i18n
					);
					return (
						<li key={entry.index} value={entry.index + 1}>
							{entry.status.kind === 'importable' ? (
								<Checkbox
									label={label}
									description={`${details.before}${details.after}`}
									checked={selectedIndexes.has(entry.index)}
									onChange={() => onToggle(entry.index)}
								/>
							) : (
								<div className="flex flex-col text-sm">
									<span className="font-medium">{label}</span>
									<span className="text-tn-muted">
										<ConfigImportTextView text={details} />
									</span>
								</div>
							)}
						</li>
					);
				})}
			</ol>
		</fieldset>
	);
}
