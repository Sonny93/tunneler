import { useState } from 'react';
import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';
import { useLingui } from '@lingui/react/macro';

import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import type { TargetHealthDto, TunnelDto } from '#/daemon/protocol';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';

type UseCheckTargetReturn = {
	isChecking: boolean;
	check: () => void;
};

function describeHealth(
	health: TargetHealthDto,
	i18n: I18n
): {
	tone: 'success' | 'error';
	message: string;
} {
	switch (health.kind) {
		case 'reachable':
			return { tone: 'success', message: t(i18n)`Target reachable` };
		case 'unreachable': {
			const { error } = health;
			return {
				tone: 'error',
				message: t(i18n)`Target unreachable: ${error}`,
			};
		}
		case 'unknown':
			return {
				tone: 'error',
				message: t(i18n)`Target check inconclusive`,
			};
	}
}

/** Tries the target of a tunnel once on demand and reports the outcome as a toast. */
export function useCheckTarget(
	tunnel: Readonly<TunnelDto>
): UseCheckTargetReturn {
	const { i18n } = useLingui();
	const gateway = useDaemonGateway();
	const [isChecking, setIsChecking] = useState(false);
	const { id } = tunnel;

	const check = () => {
		if (isChecking) return;
		setIsChecking(true);
		void (async () => {
			try {
				const health = await gateway.checkTarget(id);
				useToastStore.getState().pushToast(describeHealth(health, i18n));
			} catch (rejection) {
				const errorMessage = describeDaemonError(
					toDaemonCommandError(rejection),
					i18n
				);
				useToastStore.getState().pushToast({
					tone: 'error',
					message: t(i18n)`Couldn't check the target: ${errorMessage}`,
				});
			} finally {
				setIsChecking(false);
			}
		})();
	};

	return { isChecking, check };
}
