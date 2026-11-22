import { msg, plural } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { useState, type RefObject } from 'react';

import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';

const SUGGESTED_FILE_NAME = 'tunneler-export.toml';
const NO_MATCH_MESSAGE = msg`No saved tunnel matches these filters.`;

type UseConfigExportReturn = {
	hosts: readonly string[];
	tags: readonly string[];
	isExporting: boolean;
	/** What the last export did, for sighted users; the same text is announced. */
	resultText: string | null;
	summary: ReturnType<typeof useFormErrorSummary>;
	changeHosts: (hosts: readonly string[]) => void;
	changeTags: (tags: readonly string[]) => void;
	exportToFile: () => void;
};

/** `buttonRef` is the export button: it takes the focus back when the native dialog closes. */
export function useConfigExport(
	buttonRef: RefObject<HTMLButtonElement | null>
): UseConfigExportReturn {
	const { i18n } = useLingui();
	const gateway = useDaemonGateway();
	const [hosts, setHosts] = useState<readonly string[]>([]);
	const [tags, setTags] = useState<readonly string[]>([]);
	const [isExporting, setIsExporting] = useState(false);
	const [resultText, setResultText] = useState<string | null>(null);
	const summary = useFormErrorSummary();

	const runExport = async () => {
		const { contents, tunnelCount } = await gateway.exportConfig(hosts, tags);
		if (tunnelCount === 0) {
			summary.reportMessagesKeepingFocus([i18n._(NO_MATCH_MESSAGE)]);
			return;
		}
		const { saved } = await gateway.exportToFile(contents, SUGGESTED_FILE_NAME);
		if (!saved) return;
		const text = i18n._(
			msg`${plural(tunnelCount, {
				one: '# tunnel exported',
				other: '# tunnels exported',
			})}`
		);
		setResultText(text);
		useAnnouncerStore.getState().announce(text, 'polite');
	};

	const exportToFile = async () => {
		if (isExporting) return;
		setIsExporting(true);
		setResultText(null);
		summary.clearFailure();
		try {
			await runExport();
		} catch (rejection) {
			summary.reportFailure(rejection);
		} finally {
			setIsExporting(false);
			buttonRef.current?.focus();
		}
	};

	return {
		hosts,
		tags,
		isExporting,
		resultText,
		summary,
		changeHosts: setHosts,
		changeTags: setTags,
		exportToFile: () => void exportToFile(),
	};
}
