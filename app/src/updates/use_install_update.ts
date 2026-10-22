import { useCallback } from 'react';
import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';
import { useLingui } from '@lingui/react/macro';

import { useDaemonStore } from '#/state/daemon_store';
import { useUpdateStore } from '#/updates/update_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { confirmTunnelsStop } from '#/quit/confirm_tunnels_stop';
import { countRunningSessions } from '#/quit/count_running_sessions';
import { createProgressAnnouncer } from '#/updates/create_progress_announcer';

function describeConsequence(isRestoredOnLaunch: boolean, i18n: I18n): string {
	return isRestoredOnLaunch
		? t(
				i18n
			)`They will restart after the update, because active tunnels are restored on launch.`
		: t(
				i18n
			)`They will not restart after the update: "Restore active tunnels on launch" is turned off in the settings.`;
}

async function confirmStoppingTunnels(i18n: I18n): Promise<boolean> {
	const { tunnels, x11Anchors, settings } = useDaemonStore.getState();
	const runningCount = countRunningSessions(tunnels, x11Anchors);
	if (runningCount === 0) return true;
	return confirmTunnelsStop({
		title: t(i18n)`Install the update?`,
		confirmLabel: t(i18n)`Stop everything and install`,
		runningCount,
		consequence: describeConsequence(settings?.restoreOnLaunch ?? false, i18n),
	});
}

/**
 * Installs the pending update: asks for confirmation when tunnels run,
 * follows the download in the update store, and reports a failure as an
 * error toast. On success the native side exits the app.
 */
export function useInstallUpdate(): () => Promise<void> {
	const { i18n } = useLingui();
	const run = useDaemonAction();

	return useCallback(async () => {
		if (!(await confirmStoppingTunnels(i18n))) return;
		const { startInstallation, reportProgress, endInstallation } =
			useUpdateStore.getState();
		const announcer = createProgressAnnouncer(i18n);
		startInstallation();
		announcer.start();
		const isInstalled = await run(
			(gateway) =>
				gateway.installUpdate((progress) => {
					reportProgress(progress);
					announcer.report(progress);
				}),
			t(i18n)`Couldn't install the update`
		);
		if (!isInstalled) endInstallation();
	}, [run, i18n]);
}
