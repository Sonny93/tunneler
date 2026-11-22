import { useLingui } from '@lingui/react/macro';
import { Drawer, Tabs } from '@minimalstuff/ui';
import { useState } from 'react';

import type { SettingsDto } from '#/daemon/protocol';
import { DevslotResult } from '#/import/devslot/devslot_result';
import { ImportDevslotTab } from '#/import/devslot/import_devslot_tab';
import { useDevslotDiscovery } from '#/import/devslot/use_devslot_discovery';
import { ImportConfigTab } from '#/import/import_config_tab';
import { ImportPreview } from '#/import/import_preview';
import { ImportScriptTab } from '#/import/import_script_tab';
import { useImportAnalysis } from '#/import/use_import_analysis';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useUiStore } from '#/state/ui_store';

type ImportPanelProps = {
	title: string;
	settings: SettingsDto;
	isOpen: boolean;
};

export function ImportPanel({
	title,
	settings,
	isOpen,
}: Readonly<ImportPanelProps>) {
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
	];

	return (
		<Drawer isOpen={isOpen} title={title} onClose={closePanel}>
			<div className="flex flex-col gap-6">
				<Tabs items={tabs} />
				<FormErrorSummary
					messages={summary.messages}
					title={t`Analysis failed:`}
					attach={summary.attachSummary}
				/>
				<FormErrorSummary
					messages={devslot.summary.messages}
					title={devslot.summaryTitle}
					attach={devslot.summary.attachSummary}
				/>
				{analysis !== null && (
					<ImportPreview
						key={analysis.revision}
						preview={analysis.preview}
						settings={settings}
					/>
				)}
				{devslot.discovery !== null && (
					<DevslotResult
						key={devslot.discovery.revision}
						discovery={devslot.discovery.result}
						settings={settings}
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
