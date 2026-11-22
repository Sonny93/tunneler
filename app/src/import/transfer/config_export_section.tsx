import { useLingui } from '@lingui/react/macro';
import { Button, CheckboxOptions } from '@minimalstuff/ui';
import { useId, useMemo, useRef } from 'react';

import { useConfigExport } from '#/import/transfer/use_config_export';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { summarizeTags } from '#/sidebar/summarize_tags';
import { useDaemonStore } from '#/state/daemon_store';
import { compareTextCaseInsensitively } from '#/tunnels/compare_text_case_insensitively';

export function ConfigExportSection() {
	const { t } = useLingui();
	const headingId = useId();
	const tunnels = useDaemonStore((state) => state.tunnels);
	const exportButtonRef = useRef<HTMLButtonElement>(null);
	const exporter = useConfigExport(exportButtonRef);
	const hostOptions = useMemo(
		() =>
			[
				...new Set(
					tunnels
						.filter((tunnel) => !tunnel.isTemporary)
						.map(({ host }) => host)
				),
			].sort(compareTextCaseInsensitively),
		[tunnels]
	);
	const tagOptions = useMemo(
		() => summarizeTags(tunnels).map(({ tag }) => tag),
		[tunnels]
	);

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-3">
			<h3 id={headingId} className="m-0 text-base font-semibold">
				{t`Export to a file`}
			</h3>
			<p className="m-0 text-sm text-tn-muted">
				{t`Saves your tunnels to a file you can import on another machine. Settings, openers and passwords are not included. Without a filter, every saved tunnel is exported.`}
			</p>
			{hostOptions.length > 0 && (
				<CheckboxOptions
					label={t`Only these hosts (optional)`}
					options={hostOptions}
					values={exporter.hosts}
					onChange={exporter.changeHosts}
				/>
			)}
			{tagOptions.length > 0 && (
				<CheckboxOptions
					label={t`Only these tags (optional)`}
					options={tagOptions}
					values={exporter.tags}
					onChange={exporter.changeTags}
				/>
			)}
			<FormErrorSummary
				messages={exporter.summary.messages}
				englishTexts={exporter.summary.englishTexts}
				title={t`Export failed:`}
				attach={exporter.summary.attachSummary}
			/>
			<div>
				<Button
					ref={exportButtonRef}
					startIcon="i-mdi-export"
					loading={exporter.isExporting}
					onClick={exporter.exportToFile}
				>
					{t`Export…`}
				</Button>
			</div>
			{exporter.resultText !== null && (
				<p className="m-0 text-sm">{exporter.resultText}</p>
			)}
		</section>
	);
}
