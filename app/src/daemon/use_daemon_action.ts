import { useCallback } from 'react';

import { useToastStore } from '#/shared/toasts/toast_store';
import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';

type DaemonAction = (gateway: DaemonGateway) => Promise<unknown>;

/**
 * Runs a daemon call and reports a failure as an error toast:
 * "{failureMessage} : {daemon message}". Resolves to whether the call
 * succeeded, so callers can chain on success without handling errors.
 */
export type RunDaemonAction = (
	action: DaemonAction,
	failureMessage: string
) => Promise<boolean>;

export function useDaemonAction(): RunDaemonAction {
	const gateway = useDaemonGateway();

	return useCallback(
		async (action, failureMessage) => {
			try {
				await action(gateway);
				return true;
			} catch (rejection) {
				const { message } = toDaemonCommandError(rejection);
				useToastStore.getState().pushToast({
					tone: 'error',
					message: `${failureMessage} : ${message}`,
				});
				return false;
			}
		},
		[gateway]
	);
}
