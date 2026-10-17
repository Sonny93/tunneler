import { useState } from 'react';
import { Tabs } from '@minimalstuff/ui';

import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import type { SettingsDto } from '#/daemon/protocol';
import { ImportPreview } from '#/import/import_preview';
import { ImportConfigTab } from '#/import/import_config_tab';
import { ImportScriptTab } from '#/import/import_script_tab';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useImportAnalysis } from '#/import/use_import_analysis';

type ImportPanelProps = {
	title: string;
	settings: SettingsDto;
};

export function ImportPanel({ title, settings }: Readonly<ImportPanelProps>) {
	const closePanel = useUiStore((state) => state.closePanel);
	const [scriptText, setScriptText] = useState('');
	const { analysis, isAnalyzing, summary, analyzeScript, analyzeSshConfig } =
		useImportAnalysis();

	const tabs = [
		{
			title: 'Coller un script',
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
			title: 'Depuis ~/.ssh/config',
			content: (
				<ImportConfigTab
					isAnalyzing={isAnalyzing}
					onAnalyze={analyzeSshConfig}
				/>
			),
		},
	];

	return (
		<Drawer isOpen title={title} onClose={closePanel}>
			<div className="flex flex-col gap-6">
				<Tabs items={tabs} />
				<FormErrorSummary
					messages={summary.messages}
					title="Analyse impossible :"
					attach={summary.attachSummary}
				/>
				{analysis !== null && (
					<ImportPreview
						key={analysis.revision}
						preview={analysis.preview}
						settings={settings}
					/>
				)}
			</div>
		</Drawer>
	);
}
