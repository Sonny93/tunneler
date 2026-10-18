import { Button, Input, Select } from '@minimalstuff/ui';
import { useEffect, useId, useRef, type ChangeEvent } from 'react';

import { FormHint } from '#/shared/form_hint';
import { useDaemonStore } from '#/state/daemon_store';
import { useImportForm } from '#/import/use_import_form';
import { ImportWarnings } from '#/import/import_warnings';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { ImportCandidateList } from '#/import/import_candidate_list';
import type { ImportPreviewDto, SettingsDto } from '#/daemon/protocol';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import {
	describeDetectedCount,
	describeImportAction,
} from '#/import/import_texts';

type ImportPreviewProps = {
	preview: ImportPreviewDto;
	settings: SettingsDto;
};

/** Mount it with a new `key` per analysis: it announces its count and takes the focus when mounted. */
export function ImportPreview({
	preview,
	settings,
}: Readonly<ImportPreviewProps>) {
	const headingId = useId();
	const tagsHintId = useId();
	const headingRef = useRef<HTMLHeadingElement>(null);
	const hosts = useDaemonStore((state) => state.hosts);
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
			.announce(describeDetectedCount(candidates.length), 'polite');
	}, [candidates.length]);

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
				Résultat de l'analyse
			</h3>
			<p className="m-0 text-sm">{describeDetectedCount(candidates.length)}</p>
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
							label="Hôte des tunnels sans destination"
							options={hostOptions}
							placeholder="Choisir un hôte"
							value={form.defaultHost}
							onChange={handleHostChange}
							error={form.hostError}
							required
						/>
					)}
					<div>
						<Input
							name="tags"
							label="Tags ajoutés aux tunnels importés"
							value={form.tagsText}
							onChange={handleTagsChange}
							autoComplete="off"
							aria-describedby={tagsHintId}
						/>
						<FormHint id={tagsHintId}>Séparés par des virgules.</FormHint>
					</div>
					<ImportCandidateList
						candidates={candidates}
						names={form.names}
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
								Import en cours : {form.progress.doneCount} sur{' '}
								{form.progress.totalCount}
							</output>
						)}
						<div>
							<Button
								type="submit"
								startIcon="i-mdi-import"
								loading={form.progress !== null}
								disabled={selectedCount === 0}
							>
								{describeImportAction(selectedCount)}
							</Button>
						</div>
					</div>
				</form>
			)}
		</section>
	);
}
