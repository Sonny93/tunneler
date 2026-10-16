import { ConfirmModal } from '@minimalstuff/ui';

import type { ProcessOwnerDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { describeProcess } from '#/externals/describe_process';

/** Asks for confirmation, spelling out `consequence`, then terminates the process. */
type ConfirmKillProcess = (
	process: Readonly<ProcessOwnerDto>,
	consequence: string
) => Promise<void>;

export function useKillProcess(): ConfirmKillProcess {
	const runDaemonAction = useDaemonAction();

	return async (process, consequence) => {
		const description = describeProcess(process);
		const isConfirmed = await ConfirmModal.call({
			title: `Arrêter ${description} ?`,
			children: consequence,
			confirmLabel: 'Arrêter le processus',
			cancelLabel: 'Annuler',
			closeLabel: 'Fermer',
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		const isKilled = await runDaemonAction(
			(gateway) => gateway.killExternal(process.pid),
			'Arrêt du processus impossible'
		);
		if (!isKilled) return;
		useToastStore.getState().pushToast({
			tone: 'success',
			message: `Processus ${description} arrêté`,
		});
	};
}
