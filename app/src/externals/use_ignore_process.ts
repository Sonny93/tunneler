import { useLingui } from '@lingui/react/macro';

import type { ProcessOwnerDto } from '#/daemon/protocol';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { describeProcess } from '#/externals/describe_process';
import { pushToast } from '#/shared/toasts/push_toast';

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
		pushToast({
			tone: 'success',
			message: t`Process ${description} ignored`,
		});
	};
}
