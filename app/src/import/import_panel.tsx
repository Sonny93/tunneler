import { useLingui } from '@lingui/react/macro';
import { Drawer, Tabs } from '@minimalstuff/ui';
import { useState } from 'react';

import { DevslotResult } from '#/import/devslot/devslot_result';
import { ImportDevslotTab } from '#/import/devslot/import_devslot_tab';
import { useDevslotDiscovery } from '#/import/devslot/use_devslot_discovery';
import { ImportConfigTab } from '#/import/import_config_tab';
import { ImportPreview } from '#/import/import_preview';
import { ImportScriptTab } from '#/import/import_script_tab';
import { ImportFileTab } from '#/import/transfer/import_file_tab';
import { useImportAnalysis } from '#/import/use_import_analysis';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useUiStore } from '#/state/ui_store';

type ImportPanelProps = {
	title: string;
	isOpen: boolean;
};

/** The sticky action bar of the forms is about 7rem high: keeps a focused field from hiding under it (WCAG 2.4.11). */
const ACTION_BAR_SCROLL_PADDING = 'scroll-pb-28';

export function ImportPanel({ title, isOpen }: Readonly<ImportPanelProps>) {
	const { t } = useLingui();
	const closePanel = useUiStore((state) => state.closePanel);
	const [scriptText, setScriptText] = useState('');
	const { analysis, isAnalyzing, summary, analyzeScript, analyzeSshConfig } =
		useImportAnalysis();
	const devslot = useDevslotDiscovery();

	const tabs = [
		{
			title: t`Paste a script`,
			content: (
				<ImportScriptTab
					text={scriptText}
					isAnalyzing={isAnalyzing}
					onTextChange={setScriptText}
					onAnalyze={analyzeScript}
				/>
			),
		},
		{
			title: t`From ~/.ssh/config`,
			content: (
				<ImportConfigTab
					isAnalyzing={isAnalyzing}
					onAnalyze={analyzeSshConfig}
				/>
			),
		},
		{
			title: t`From devslot`,
			content: (
				<ImportDevslotTab
					isDiscovering={devslot.isDiscovering}
					notFoundHost={devslot.notFoundHost}
					statusMessage={devslot.statusMessage}
					onDiscover={devslot.discover}
				/>
			),
		},
		{ title: t`File`, content: <ImportFileTab /> },
	];

	return (
		<Drawer
			isOpen={isOpen}
			title={title}
			onClose={closePanel}
			className={ACTION_BAR_SCROLL_PADDING}
		>
			<div className="flex flex-col gap-6">
				<Tabs items={tabs} />
				<FormErrorSummary
					messages={summary.messages}
					englishTexts={summary.englishTexts}
					title={t`Analysis failed:`}
					attach={summary.attachSummary}
				/>
				<FormErrorSummary
					messages={devslot.summary.messages}
					englishTexts={devslot.summary.englishTexts}
					title={devslot.summaryTitle}
					attach={devslot.summary.attachSummary}
				/>
				{analysis !== null && (
					<ImportPreview key={analysis.revision} preview={analysis.preview} />
				)}
				{devslot.discovery !== null && (
					<DevslotResult
						key={devslot.discovery.revision}
						discovery={devslot.discovery.result}
						removedIds={devslot.removedIds}
						onStatus={devslot.reportStatus}
						onRemoved={devslot.reportRemoved}
						onFailure={devslot.reportFailureWithTitle}
					/>
				)}
			</div>
		</Drawer>
	);
}
