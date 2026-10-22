import { useCallback } from 'react';
import { useLingui } from '@lingui/react/macro';

import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { confirmTunnelsStop } from '#/quit/confirm_tunnels_stop';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';

const TUNNELS_RUNNING_CODE = 'tunnels_running';

type UseQuitFlowReturn = {
	/** Quits, asking for confirmation when the daemon reports running tunnels. */
	quit: () => Promise<void>;
	/** Asks for confirmation for `runningCount` tunnels, then force-quits. */
	confirmAndQuit: (runningCount: number) => Promise<void>;
};

function readRunningCount(data: unknown): number {
	return typeof data === 'number' ? data : 0;
}

export function useQuitFlow(): UseQuitFlowReturn {
	const { t, i18n } = useLingui();
	const gateway = useDaemonGateway();
	const run = useDaemonAction();

	const confirmAndQuit = useCallback(
		async (runningCount: number) => {
			const isConfirmed = await confirmTunnelsStop({
				title: t`Quit Tunneler?`,
				confirmLabel: t`Stop everything and quit`,
				runningCount,
			});
			if (isConfirmed)
				await run((daemon) => daemon.quit(true), t`Couldn't quit`);
		},
		[run, t]
	);

	const quit = useCallback(async () => {
		try {
			await gateway.quit(false);
		} catch (rejection) {
			const error = toDaemonCommandError(rejection);
			if (error.code === TUNNELS_RUNNING_CODE) {
				await confirmAndQuit(readRunningCount(error.data));
				return;
			}
			const failureMessage = t`Couldn't quit`;
			const errorMessage = describeDaemonError(error, i18n);
			useToastStore.getState().pushToast({
				tone: 'error',
				message: t`${failureMessage}: ${errorMessage}`,
			});
		}
	}, [gateway, confirmAndQuit, i18n, t]);

	return { quit, confirmAndQuit };
}
