import { useLingui } from '@lingui/react/macro';
import {
	useEffect,
	useEffectEvent,
	useId,
	useRef,
	type SubmitEvent,
} from 'react';

import type { ConfigImportPlanDto } from '#/daemon/protocol';
import { ImportActionButton } from '#/import/import_action_button';
import {
	describeDetectedCount,
	describeImportAction,
} from '#/import/import_texts';
import { ConfigImportEntryList } from '#/import/transfer/config_import_entry_list';
import { HostAliasSelect } from '#/import/transfer/host_alias_select';
import type { useImportSelection } from '#/import/use_import_selection';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useDaemonStore } from '#/state/daemon_store';

type ConfigImportPreviewProps = {
	fileName: string;
	plan: ConfigImportPlanDto;
	hostMap: Readonly<Record<string, string>>;
	remapHosts: readonly string[];
	selection: ReturnType<typeof useImportSelection>;
	isPreviewing: boolean;
	/** The host map chosen is not the one the preview was made with: an earlier preview failed. */
	isStale: boolean;
	isApplying: boolean;
	onChangeAlias: (fileHost: string, alias: string) => void;
	onApply: () => void;
};

/** Mount it with a new `key` per file: it announces its count and takes the focus when mounted. */
export function ConfigImportPreview({
	fileName,
	plan,
	hostMap,
	remapHosts,
	selection,
	isPreviewing,
	isStale,
	isApplying,
	onChangeAlias,
	onApply,
}: Readonly<ConfigImportPreviewProps>) {
	const { t, i18n } = useLingui();
	const headingId = useId();
	const headingRef = useRef<HTMLHeadingElement>(null);
	const aliases = useDaemonStore((state) => state.hosts);
	const entryCount = plan.entries.length;
	const selectedCount = selection.selectedIndexes.size;
	const checkingHint = isPreviewing
		? t`Updating the preview…`
		: t`The preview is out of date: choose the alias again`;

	const announceArrival = useEffectEvent(() => {
		headingRef.current?.focus();
		useAnnouncerStore
			.getState()
			.announce(describeDetectedCount(entryCount, i18n), 'polite');
	});
	useEffect(() => {
		announceArrival();
	}, []);

	const handleSelectAll = () =>
		selection.replace(
			plan.entries
				.filter((entry) => entry.status.kind === 'importable')
				.map((entry) => entry.index)
		);
	const handleSubmit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (selectedCount > 0 && !isPreviewing && !isStale) onApply();
	};

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-4">
			<h4
				id={headingId}
				ref={headingRef}
				tabIndex={-1}
				className="m-0 text-base font-semibold"
			>
				{t`Preview of ${fileName}`}
			</h4>
			<p className="m-0 text-sm">{describeDetectedCount(entryCount, i18n)}</p>
			<form
				noValidate
				onSubmit={handleSubmit}
				aria-busy={isApplying || isPreviewing}
				className="flex flex-col gap-4"
			>
				{remapHosts.map((fileHost) => (
					<HostAliasSelect
						key={fileHost}
						fileHost={fileHost}
						alias={hostMap[fileHost] ?? ''}
						aliases={aliases}
						isBusy={isPreviewing}
						onChange={onChangeAlias}
					/>
				))}
				{entryCount > 0 && (
					<ConfigImportEntryList
						entries={plan.entries}
						selectedIndexes={selection.selectedIndexes}
						onToggle={selection.toggle}
						onSelectAll={handleSelectAll}
						onSelectNone={selection.selectNone}
					/>
				)}
				<div className="sticky bottom-0 border-t border-tn-divider bg-tn-surface py-3">
					<ImportActionButton
						startIcon="i-mdi-import"
						isRunning={isApplying}
						isChecking={isPreviewing || isStale}
						checkingHint={checkingHint}
						isSelectionEmpty={selectedCount === 0}
						emptyHint={t`Select at least one tunnel`}
					>
						{describeImportAction(selectedCount, i18n)}
					</ImportActionButton>
				</div>
			</form>
		</section>
	);
}
