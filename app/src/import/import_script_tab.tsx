import { Button, Textarea } from '@minimalstuff/ui';
import { useId, useState, type ChangeEvent } from 'react';

import { FormHint } from '#/shared/form_hint';

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
	const hintId = useId();
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
			<div>
				<Textarea
					name="script"
					label="Script, liste de ports ou options -L/-R/-D"
					value={text}
					onChange={handleChange}
					error={error}
					rows={8}
					required
					spellCheck={false}
					autoComplete="off"
					aria-describedby={hintId}
					className="font-mono"
				/>
				<FormHint id={hintId}>
					Script PowerShell ou shell (ssh -L …), liste de ports (3333, 5432), ou
					une option par ligne.
				</FormHint>
			</div>
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
