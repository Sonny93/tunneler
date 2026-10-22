import { useLingui } from '@lingui/react/macro';

import type { ProcessOwnerDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { describeProcess } from '#/externals/describe_process';

type IgnoreProcess = (process: Readonly<ProcessOwnerDto>) => Promise<void>;

export function useIgnoreProcess(): IgnoreProcess {
	const { t } = useLingui();
	const runDaemonAction = useDaemonAction();

	return async (process) => {
		const isIgnored = await runDaemonAction(
			(gateway) => gateway.ignoreExternal(process.pid),
			t`Couldn't ignore the process`
		);
		if (!isIgnored) return;
		const description = describeProcess(process);
		useToastStore.getState().pushToast({
			tone: 'success',
			message: t`Process ${description} ignored`,
		});
	};
}
