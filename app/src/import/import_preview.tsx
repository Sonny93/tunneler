import { useLingui } from '@lingui/react/macro';
import { Button, Input, Select } from '@minimalstuff/ui';
import { useEffect, useId, useMemo, useRef, type ChangeEvent } from 'react';

import type { ImportPreviewDto, SettingsDto } from '#/daemon/protocol';
import { ImportCandidateList } from '#/import/import_candidate_list';
import {
	describeDetectedCount,
	describeImportAction,
	describeImportProgress,
} from '#/import/import_texts';
import { ImportWarnings } from '#/import/import_warnings';
import { useImportForm } from '#/import/use_import_form';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { summarizeTags } from '#/sidebar/summarize_tags';
import { useDaemonStore } from '#/state/daemon_store';

type ImportPreviewProps = {
	preview: ImportPreviewDto;
	settings: SettingsDto;
};

/** Mount it with a new `key` per analysis: it announces its count and takes the focus when mounted. */
export function ImportPreview({
	preview,
	settings,
}: Readonly<ImportPreviewProps>) {
	const { t, i18n } = useLingui();
	const headingId = useId();
	const tagsListId = useId();
	const headingRef = useRef<HTMLHeadingElement>(null);
	const hosts = useDaemonStore((state) => state.hosts);
	const tunnels = useDaemonStore((state) => state.tunnels);
	const existingTags = useMemo(() => summarizeTags(tunnels), [tunnels]);
	const form = useImportForm(preview, settings);
	const { candidates, warnings } = preview;
	const selectedCount = form.selection.selectedIndexes.size;
	const hasHostlessCandidate = candidates.some(
		(candidate) => candidate.host === undefined
	);
	const hostOptions = hosts.map((host) => ({ value: host, label: host }));

	useEffect(() => {
		headingRef.current?.focus();
		useAnnouncerStore
			.getState()
			.announce(describeDetectedCount(candidates.length, i18n), 'polite');
	}, [candidates.length, i18n]);

	const handleHostChange = (event: ChangeEvent<HTMLSelectElement>) =>
		form.changeDefaultHost(event.target.value);
	const handleTagsChange = (event: ChangeEvent<HTMLInputElement>) =>
		form.changeTags(event.target.value);

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-4">
			<h3
				id={headingId}
				ref={headingRef}
				tabIndex={-1}
				className="m-0 text-base font-semibold"
			>
				{t`Analysis result`}
			</h3>
			<p className="m-0 text-sm">
				{describeDetectedCount(candidates.length, i18n)}
			</p>
			<ImportWarnings warnings={warnings} />
			{candidates.length > 0 && (
				<form
					noValidate
					onSubmit={form.submit}
					aria-busy={form.progress !== null}
					className="flex flex-col gap-4"
				>
					<FormErrorSummary
						messages={form.summary.messages}
						title={form.summaryTitle}
						attach={form.summary.attachSummary}
					/>
					{hasHostlessCandidate && (
						<Select
							name="defaultHost"
							label={t`Host of the tunnels without a destination`}
							options={hostOptions}
							placeholder={t`Choose a host`}
							value={form.defaultHost}
							onChange={handleHostChange}
							error={form.hostError}
							required
						/>
					)}
					<div>
						<Input
							name="tags"
							label={t`Tag to add`}
							value={form.tagsText}
							onChange={handleTagsChange}
							error={form.tagsError}
							description={t`Optional. Added to every imported tunnel; separate several tags with commas.`}
							autoComplete="off"
							list={tagsListId}
						/>
						<datalist id={tagsListId}>
							{existingTags.map(({ tag }) => (
								<option key={tag} value={tag}>
									{tag}
								</option>
							))}
						</datalist>
					</div>
					<ImportCandidateList
						candidates={candidates}
						conflicts={form.conflicts}
						selectedIndexes={form.selection.selectedIndexes}
						fallbackHost={form.defaultHost}
						onToggle={form.selection.toggle}
						onSelectAll={form.selection.selectAll}
						onSelectNone={form.selection.selectNone}
					/>
					<div className="sticky bottom-0 flex flex-col gap-2 border-t border-tn-divider bg-tn-surface py-3">
						{form.progress !== null && (
							<output className="block text-sm">
								{describeImportProgress(
									form.progress.doneCount,
									form.progress.totalCount,
									i18n
								)}
							</output>
						)}
						<div>
							<Button
								type="submit"
								startIcon="i-mdi-import"
								loading={form.progress !== null}
								disabled={selectedCount === 0}
							>
								{describeImportAction(selectedCount, i18n)}
							</Button>
						</div>
					</div>
				</form>
			)}
		</section>
	);
}
