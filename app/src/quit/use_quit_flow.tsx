import { useCallback } from 'react';

import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { confirmTunnelsStop } from '#/quit/confirm_tunnels_stop';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';

const TUNNELS_RUNNING_CODE = 'tunnels_running';
const QUIT_FAILURE_MESSAGE = 'Impossible de quitter';

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
	const gateway = useDaemonGateway();
	const run = useDaemonAction();

	const confirmAndQuit = useCallback(
		async (runningCount: number) => {
			const isConfirmed = await confirmTunnelsStop({
				title: 'Quitter Tunneler ?',
				confirmLabel: 'Tout arrêter et quitter',
				runningCount,
			});
			if (isConfirmed)
				await run((daemon) => daemon.quit(true), QUIT_FAILURE_MESSAGE);
		},
		[run]
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
			useToastStore.getState().pushToast({
				tone: 'error',
				message: `${QUIT_FAILURE_MESSAGE} : ${error.message}`,
			});
		}
	}, [gateway, confirmAndQuit]);

	return { quit, confirmAndQuit };
}
