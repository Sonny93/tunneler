import type { MessageDescriptor } from '@lingui/core';
import { useLingui } from '@lingui/react/macro';
import { ConfirmModal } from '@minimalstuff/ui';

import type { ProcessOwnerDto } from '#/daemon/protocol';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { describeProcess } from '#/externals/describe_process';
import { pushToast } from '#/shared/toasts/push_toast';

/** Asks for confirmation, spelling out `consequence`, then terminates the process. */
type ConfirmKillProcess = (
	process: Readonly<ProcessOwnerDto>,
	consequence: MessageDescriptor
) => Promise<void>;

export function useKillProcess(): ConfirmKillProcess {
	const { t, i18n } = useLingui();
	const runDaemonAction = useDaemonAction();

	return async (process, consequence) => {
		const description = describeProcess(process);
		const isConfirmed = await ConfirmModal.call({
			title: t`Stop ${description}?`,
			children: i18n._(consequence),
			confirmLabel: t`Stop the process`,
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		const isKilled = await runDaemonAction(
			(gateway) => gateway.killExternal(process.pid),
			t`Couldn't stop the process`
		);
		if (!isKilled) return;
		pushToast({
			tone: 'success',
			message: t`Process ${description} stopped`,
		});
	};
}
