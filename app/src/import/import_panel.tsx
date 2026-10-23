import { useLingui } from '@lingui/react/macro';
import { Tabs } from '@minimalstuff/ui';
import { useState } from 'react';

import type { SettingsDto } from '#/daemon/protocol';
import { ImportConfigTab } from '#/import/import_config_tab';
import { ImportPreview } from '#/import/import_preview';
import { ImportScriptTab } from '#/import/import_script_tab';
import { useImportAnalysis } from '#/import/use_import_analysis';
import { Drawer } from '#/shared/drawer';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useUiStore } from '#/state/ui_store';

type ImportPanelProps = {
	title: string;
	settings: SettingsDto;
};

export function ImportPanel({ title, settings }: Readonly<ImportPanelProps>) {
	const { t } = useLingui();
	const closePanel = useUiStore((state) => state.closePanel);
	const [scriptText, setScriptText] = useState('');
	const { analysis, isAnalyzing, summary, analyzeScript, analyzeSshConfig } =
		useImportAnalysis();

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
	];

	return (
		<Drawer isOpen title={title} onClose={closePanel}>
			<div className="flex flex-col gap-6">
				<Tabs items={tabs} />
				<FormErrorSummary
					messages={summary.messages}
					title={t`Analysis failed:`}
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
