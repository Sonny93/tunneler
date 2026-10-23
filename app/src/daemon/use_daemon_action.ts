import { i18n } from '@lingui/core';
import { t } from '@lingui/core/macro';
import { useCallback } from 'react';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';

type DaemonAction = (gateway: DaemonGateway) => Promise<unknown>;

/**
 * Runs a daemon call and reports a failure as an error toast:
 * "{failureMessage}: {daemon message}" (translated by error code when known). Resolves to whether the call
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
				const errorMessage = describeDaemonError(
					toDaemonCommandError(rejection),
					i18n
				);
				useToastStore.getState().pushToast({
					tone: 'error',
					message: t`${failureMessage}: ${errorMessage}`,
				});
				return false;
			}
		},
		[gateway]
	);
}
