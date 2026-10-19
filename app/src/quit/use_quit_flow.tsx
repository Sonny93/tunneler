import { useCallback } from 'react';
import { ConfirmModal } from '@minimalstuff/ui';

import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { describeTunnelsToStop } from '#/quit/describe_running_tunnels';

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
			const isConfirmed = await ConfirmModal.call({
				title: 'Quitter Tunneler ?',
				children: <p>{describeTunnelsToStop(runningCount)}</p>,
				confirmLabel: 'Tout arrêter et quitter',
				confirmColor: 'danger',
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
