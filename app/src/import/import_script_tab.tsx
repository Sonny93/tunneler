import { useState, type ChangeEvent } from 'react';
import { Button, Textarea } from '@minimalstuff/ui';

const SCRIPT_HINT =
	'Script PowerShell ou shell (ssh -L …), liste de ports (3333, 5432), ou une option par ligne.';
const EMPTY_TEXT_ERROR =
	'Collez un script, une liste de ports ou des options -L/-R/-D.';

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
	const [error, setError] = useState<string>();

	const handleChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
		setError(undefined);
		onTextChange(event.target.value);
	};
	const handleAnalyze = () => {
		if (text.trim() === '') {
			setError(EMPTY_TEXT_ERROR);
			return;
		}
		onAnalyze(text);
	};

	return (
		<div className="flex flex-col gap-3 pt-3">
			<Textarea
				name="script"
				label="Script, liste de ports ou options -L/-R/-D"
				description={SCRIPT_HINT}
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
					Analyser
				</Button>
			</div>
		</div>
	);
}
