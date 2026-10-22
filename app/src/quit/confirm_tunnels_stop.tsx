import { i18n } from '@lingui/core';
import { ConfirmModal } from '@minimalstuff/ui';

import { describeTunnelsToStop } from '#/quit/describe_running_tunnels';

type ConfirmTunnelsStopOptions = {
	title: string;
	confirmLabel: string;
	runningCount: number;
	/** What happens next, shown under the count of tunnels that will stop. */
	consequence?: string;
};

/** Asks the user to confirm stopping `runningCount` tunnels; resolves to whether they agreed. */
export function confirmTunnelsStop({
	title,
	confirmLabel,
	runningCount,
	consequence,
}: Readonly<ConfirmTunnelsStopOptions>): Promise<boolean> {
	return ConfirmModal.call({
		title,
		children: (
			<>
				<p>{describeTunnelsToStop(runningCount, i18n)}</p>
				{consequence !== undefined && <p>{consequence}</p>}
			</>
		),
		confirmLabel,
		confirmColor: 'danger',
	});
}
