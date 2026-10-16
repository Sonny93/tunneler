import type { ProcessOwnerDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { describeProcess } from '#/externals/describe_process';

type IgnoreProcess = (process: Readonly<ProcessOwnerDto>) => Promise<void>;

export function useIgnoreProcess(): IgnoreProcess {
	const runDaemonAction = useDaemonAction();

	return async (process) => {
		const isIgnored = await runDaemonAction(
			(gateway) => gateway.ignoreExternal(process.pid),
			'Impossible d’ignorer le processus'
		);
		if (!isIgnored) return;
		useToastStore.getState().pushToast({
			tone: 'success',
			message: `Processus ${describeProcess(process)} ignoré`,
		});
	};
}
