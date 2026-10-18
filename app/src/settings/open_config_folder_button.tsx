import { Button } from '@minimalstuff/ui';

import { useDaemonAction } from '#/daemon/use_daemon_action';

export function OpenConfigFolderButton() {
	const runDaemonAction = useDaemonAction();

	const handleOpen = () =>
		void runDaemonAction(
			(gateway) => gateway.revealConfig(),
			'Ouverture impossible'
		);

	return (
		<div className="mt-6">
			<Button
				startIcon="i-mdi-folder-open-outline"
				variant="outline"
				color="neutral"
				onClick={handleOpen}
			>
				Ouvrir le dossier de configuration
			</Button>
		</div>
	);
}
