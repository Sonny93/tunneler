import type { TunnelDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useKillProcess } from '#/externals/use_kill_process';
import { useIgnoreProcess } from '#/externals/use_ignore_process';
import {
	KILL_CONFLICT_CONSEQUENCE,
	KILL_EXTERNAL_CONSEQUENCE,
} from '#/externals/kill_consequences';

type UseExternalActionsReturn = {
	adopt: () => void;
	confirmKill: () => Promise<void>;
	ignore: () => Promise<void>;
};

function describeAdoption(
	tunnel: Readonly<TunnelDto>,
	adoptedIds: readonly string[]
): string {
	if (adoptedIds.length > 1) return `${adoptedIds.length} tunnels adoptés`;
	return `Tunnel ${tunnel.name} adopté`;
}

/** Actions on a tunnel held by an external ssh process or blocked by another process. */
export function useExternalActions(
	tunnel: Readonly<TunnelDto>
): UseExternalActionsReturn {
	const runDaemonAction = useDaemonAction();
	const killProcess = useKillProcess();
	const ignoreProcess = useIgnoreProcess();
	const { state } = tunnel;

	const adopt = () => {
		void runDaemonAction(async (gateway) => {
			const adoptedIds = await gateway.adoptExternal(tunnel.id);
			useToastStore.getState().pushToast({
				tone: 'success',
				message: describeAdoption(tunnel, adoptedIds),
			});
		}, 'Adoption impossible');
	};

	const confirmKill = async () => {
		if (state.kind === 'external') {
			await killProcess(state.owner, KILL_EXTERNAL_CONSEQUENCE);
		}
		if (state.kind === 'conflict' && state.owner) {
			await killProcess(state.owner, KILL_CONFLICT_CONSEQUENCE);
		}
	};

	const ignore = async () => {
		if (state.kind === 'external') await ignoreProcess(state.owner);
	};

	return { adopt, confirmKill, ignore };
}
