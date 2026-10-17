import { Button } from '@minimalstuff/ui';

type ImportConfigTabProps = {
	isAnalyzing: boolean;
	onAnalyze: () => void;
};

export function ImportConfigTab({
	isAnalyzing,
	onAnalyze,
}: Readonly<ImportConfigTabProps>) {
	return (
		<div className="flex flex-col gap-3 pt-3">
			<p className="m-0 text-sm text-tn-muted">
				Importe les options LocalForward, RemoteForward et DynamicForward
				déclarées dans <code>~/.ssh/config</code>.
			</p>
			<div>
				<Button
					startIcon="i-mdi-file-search-outline"
					loading={isAnalyzing}
					onClick={onAnalyze}
				>
					Lire ~/.ssh/config
				</Button>
			</div>
		</div>
	);
}
