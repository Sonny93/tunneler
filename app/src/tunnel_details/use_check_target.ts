import { useState } from 'react';

import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import type { TargetHealthDto, TunnelDto } from '#/daemon/protocol';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';

type UseCheckTargetReturn = {
	isChecking: boolean;
	check: () => void;
};

function describeHealth(health: TargetHealthDto): {
	tone: 'success' | 'error';
	message: string;
} {
	switch (health.kind) {
		case 'reachable':
			return { tone: 'success', message: 'Cible joignable' };
		case 'unreachable':
			return {
				tone: 'error',
				message: `Cible injoignable : ${health.error}`,
			};
		case 'unknown':
			return { tone: 'error', message: 'Test de la cible non concluant' };
	}
}

/** Tries the target of a tunnel once on demand and reports the outcome as a toast. */
export function useCheckTarget(
	tunnel: Readonly<TunnelDto>
): UseCheckTargetReturn {
	const gateway = useDaemonGateway();
	const [isChecking, setIsChecking] = useState(false);
	const { id } = tunnel;

	const check = () => {
		if (isChecking) return;
		setIsChecking(true);
		void (async () => {
			try {
				const health = await gateway.checkTarget(id);
				useToastStore.getState().pushToast(describeHealth(health));
			} catch (rejection) {
				const { message } = toDaemonCommandError(rejection);
				useToastStore.getState().pushToast({
					tone: 'error',
					message: `Test de la cible impossible : ${message}`,
				});
			} finally {
				setIsChecking(false);
			}
		})();
	};

	return { isChecking, check };
}
