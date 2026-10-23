import { Trans, useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';

type ImportConfigTabProps = {
	isAnalyzing: boolean;
	onAnalyze: () => void;
};

export function ImportConfigTab({
	isAnalyzing,
	onAnalyze,
}: Readonly<ImportConfigTabProps>) {
	const { t } = useLingui();

	return (
		<div className="flex flex-col gap-3 pt-3">
			<p className="m-0 text-sm text-tn-muted">
				<Trans>
					Imports the LocalForward, RemoteForward and DynamicForward options
					declared in <code>~/.ssh/config</code>.
				</Trans>
			</p>
			<div>
				<Button
					startIcon="i-mdi-file-search-outline"
					loading={isAnalyzing}
					onClick={onAnalyze}
				>
					{t`Read ~/.ssh/config`}
				</Button>
			</div>
		</div>
	);
}
