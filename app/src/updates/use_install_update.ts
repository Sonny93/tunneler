import { useCallback } from 'react';

import { useDaemonStore } from '#/state/daemon_store';
import { useUpdateStore } from '#/updates/update_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { confirmTunnelsStop } from '#/quit/confirm_tunnels_stop';
import { countRunningSessions } from '#/quit/count_running_sessions';
import { createProgressAnnouncer } from '#/updates/create_progress_announcer';

const INSTALL_FAILURE_MESSAGE = 'Mise à jour impossible';
const RESTORED_AFTER_UPDATE =
	'Ils redémarreront après la mise à jour, car les tunnels actifs sont rétablis au lancement.';
const NOT_RESTORED_AFTER_UPDATE =
	'Ils ne redémarreront pas après la mise à jour : « Rétablir les tunnels actifs au lancement » est désactivé dans les paramètres.';

async function confirmStoppingTunnels(): Promise<boolean> {
	const { tunnels, x11Anchors, settings } = useDaemonStore.getState();
	const runningCount = countRunningSessions(tunnels, x11Anchors);
	if (runningCount === 0) return true;
	return confirmTunnelsStop({
		title: 'Installer la mise à jour ?',
		confirmLabel: 'Tout arrêter et installer',
		runningCount,
		consequence: settings?.restoreOnLaunch
			? RESTORED_AFTER_UPDATE
			: NOT_RESTORED_AFTER_UPDATE,
	});
}

/**
 * Installs the pending update: asks for confirmation when tunnels run,
 * follows the download in the update store, and reports a failure as an
 * error toast. On success the native side exits the app.
 */
export function useInstallUpdate(): () => Promise<void> {
	const run = useDaemonAction();

	return useCallback(async () => {
		if (!(await confirmStoppingTunnels())) return;
		const { startInstallation, reportProgress, endInstallation } =
			useUpdateStore.getState();
		const announcer = createProgressAnnouncer();
		startInstallation();
		announcer.start();
		const isInstalled = await run(
			(gateway) =>
				gateway.installUpdate((progress) => {
					reportProgress(progress);
					announcer.report(progress);
				}),
			INSTALL_FAILURE_MESSAGE
		);
		if (!isInstalled) endInstallation();
	}, [run]);
}
