import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';
import { useId, useRef } from 'react';

import { ConfigImportPreview } from '#/import/transfer/config_import_preview';
import { ConfigImportResult } from '#/import/transfer/config_import_result';
import { useConfigImport } from '#/import/transfer/use_config_import';
import { FormErrorSummary } from '#/shared/form_error_summary';

export function ConfigImportSection() {
	const { t } = useLingui();
	const headingId = useId();
	const chooseButtonRef = useRef<HTMLButtonElement>(null);
	const importer = useConfigImport(chooseButtonRef);
	const { loaded, report, busy } = importer;

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-3">
			<h3 id={headingId} className="m-0 text-base font-semibold">
				{t`Import from a file`}
			</h3>
			<p className="m-0 text-sm text-tn-muted">
				{t`Reads a file exported by Tunneler, or a tunnels.toml. Tunnels are only added: nothing you already have is changed or removed. Added tunnels start stopped.`}
			</p>
			<FormErrorSummary
				messages={importer.summary.messages}
				englishTexts={importer.summary.englishTexts}
				title={t`Import failed:`}
				attach={importer.summary.attachSummary}
			/>
			<div>
				<Button
					ref={chooseButtonRef}
					startIcon="i-mdi-file-import-outline"
					loading={busy === 'picking'}
					onClick={importer.chooseFile}
				>
					{t`Choose a file…`}
				</Button>
			</div>
			{loaded !== null && (
				<ConfigImportPreview
					key={loaded.revision}
					fileName={loaded.fileName}
					plan={loaded.plan}
					hostMap={loaded.hostMap}
					remapHosts={loaded.remapHosts}
					selection={importer.selection}
					isPreviewing={busy === 'previewing'}
					isStale={loaded.hostMap !== loaded.plannedHostMap}
					isApplying={busy === 'applying'}
					onChangeAlias={importer.changeAlias}
					onApply={importer.apply}
				/>
			)}
			{report !== null && (
				<ConfigImportResult key={report.revision} result={report} />
			)}
		</section>
	);
}
