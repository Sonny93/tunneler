import { useLingui } from '@lingui/react/macro';
import { useEffect, useId, useRef } from 'react';

import { describeImportedCount } from '#/import/import_texts';
import { ConfigImportTextView } from '#/import/transfer/config_import_text';
import {
	describeConfigImportStatus,
	describeSkippedLead,
} from '#/import/transfer/describe_config_import';
import type { ConfigImportReport } from '#/import/transfer/use_config_import';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useDaemonStore } from '#/state/daemon_store';

type ConfigImportResultProps = {
	result: ConfigImportReport;
};

/** Mount it with a new `key` per import: it announces the outcome and takes the focus when mounted. */
export function ConfigImportResult({
	result,
}: Readonly<ConfigImportResultProps>) {
	const { t, i18n } = useLingui();
	const headingId = useId();
	const headingRef = useRef<HTMLHeadingElement>(null);
	const tunnels = useDaemonStore((state) => state.tunnels);
	const { fileName, report, plan } = result;
	const summary = describeImportedCount(report.added.length, i18n);

	useEffect(() => {
		headingRef.current?.focus();
		useAnnouncerStore.getState().announce(summary, 'polite');
	}, [summary]);

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-2">
			<h4
				id={headingId}
				ref={headingRef}
				tabIndex={-1}
				className="m-0 text-base font-semibold"
			>
				{t`Import of ${fileName}`}
			</h4>
			<p className="m-0 text-sm">{summary}</p>
			{report.skipped.length > 0 && (
				<ol className="m-0 flex list-decimal flex-col gap-1 pl-8 text-sm">
					{report.skipped.map(({ index, status }) => {
						const position = index + 1;
						const reason = describeConfigImportStatus(
							status,
							{ tunnels, entries: plan.entries },
							i18n
						);
						const entry = plan.entries.find(
							(candidate) => candidate.index === index
						);
						const lead =
							entry === undefined
								? t`Tunnel ${position} of the file skipped: `
								: describeSkippedLead(entry, i18n);
						return (
							<li key={index} value={position}>
								{lead}
								<ConfigImportTextView text={reason} />
							</li>
						);
					})}
				</ol>
			)}
		</section>
	);
}
