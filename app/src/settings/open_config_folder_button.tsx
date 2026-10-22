import { Button } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { useDaemonAction } from '#/daemon/use_daemon_action';

export function OpenConfigFolderButton() {
	const { t } = useLingui();
	const runDaemonAction = useDaemonAction();

	const handleOpen = () =>
		void runDaemonAction(
			(gateway) => gateway.revealConfig(),
			t`Couldn't open the folder`
		);

	return (
		<div className="mt-6">
			<Button
				startIcon="i-mdi-folder-open-outline"
				variant="outline"
				color="neutral"
				onClick={handleOpen}
			>
				{t`Open the configuration folder`}
			</Button>
		</div>
	);
}
