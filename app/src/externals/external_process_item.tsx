import { useId } from 'react';
import { Button } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import type { ExternalProcessDto } from '#/daemon/protocol';
import { useKillProcess } from '#/externals/use_kill_process';
import { useIgnoreProcess } from '#/externals/use_ignore_process';
import { ExternalForwardItem } from '#/externals/external_forward_item';
import { KILL_EXTERNAL_CONSEQUENCE } from '#/externals/kill_consequences';
import {
	describeProcess,
	describeProcessHeading,
} from '#/externals/describe_process';

type ExternalProcessItemProps = {
	process: ExternalProcessDto;
};

export function ExternalProcessItem({
	process,
}: Readonly<ExternalProcessItemProps>) {
	const { t, i18n } = useLingui();
	const headingId = useId();
	const killProcess = useKillProcess();
	const ignoreProcess = useIgnoreProcess();
	const description = describeProcess(process);

	const handleKill = () => void killProcess(process, KILL_EXTERNAL_CONSEQUENCE);
	const handleIgnore = () => void ignoreProcess(process);

	return (
		<li
			aria-labelledby={headingId}
			className="flex flex-col gap-2 rounded-md border border-tn-divider bg-tn-surface px-3 py-2"
		>
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				{describeProcessHeading(process, i18n)}
			</h3>
			<ul role="list" className="m-0 flex list-none flex-col gap-1 p-0">
				{process.forwards.map((forward) => (
					<ExternalForwardItem
						key={`${forward.kind}:${forward.listenAddress ?? ''}:${forward.listenPort}`}
						process={process}
						forward={forward}
					/>
				))}
			</ul>
			<div className="flex flex-wrap gap-2">
				<Button
					size="sm"
					variant="outline"
					color="danger"
					aria-label={t`Stop the process: ${description}`}
					onClick={handleKill}
				>
					{t`Stop the process…`}
				</Button>
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					aria-label={t`Ignore ${description}`}
					onClick={handleIgnore}
				>
					{t`Ignore`}
				</Button>
			</div>
		</li>
	);
}
