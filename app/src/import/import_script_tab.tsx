import { useLingui } from '@lingui/react/macro';
import { useState, type ChangeEvent } from 'react';
import { Button, Textarea } from '@minimalstuff/ui';

type ImportScriptTabProps = {
	text: string;
	isAnalyzing: boolean;
	onTextChange: (text: string) => void;
	onAnalyze: (text: string) => void;
};

export function ImportScriptTab({
	text,
	isAnalyzing,
	onTextChange,
	onAnalyze,
}: Readonly<ImportScriptTabProps>) {
	const { t } = useLingui();
	const [error, setError] = useState<string>();

	const handleChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
		setError(undefined);
		onTextChange(event.target.value);
	};
	const handleAnalyze = () => {
		if (text.trim() === '') {
			setError(t`Paste a script, a list of ports or -L/-R/-D options.`);
			return;
		}
		onAnalyze(text);
	};

	return (
		<div className="flex flex-col gap-3 pt-3">
			<Textarea
				name="script"
				label={t`Script, list of ports or -L/-R/-D options`}
				description={t`PowerShell or shell script (ssh -L …), list of ports (3333, 5432), or one option per line.`}
				value={text}
				onChange={handleChange}
				error={error}
				rows={8}
				required
				spellCheck={false}
				autoComplete="off"
				className="font-mono"
			/>
			<div>
				<Button
					startIcon="i-mdi-text-search"
					loading={isAnalyzing}
					onClick={handleAnalyze}
				>
					{t`Analyze`}
				</Button>
			</div>
		</div>
	);
}
