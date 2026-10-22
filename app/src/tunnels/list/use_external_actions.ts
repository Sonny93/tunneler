import type { I18n } from '@lingui/core';
import { plural, t } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';

import type { TunnelDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useKillProcess } from '#/externals/use_kill_process';
import { useIgnoreProcess } from '#/externals/use_ignore_process';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';
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
	adoptedIds: readonly string[],
	i18n: I18n
): string {
	const adoptedCount = adoptedIds.length;
	if (adoptedCount > 1) {
		return t(
			i18n
		)`${plural(adoptedCount, { one: '# tunnel adopted', other: '# tunnels adopted' })}`;
	}
	const endpoint = getTunnelEndpointLabel(tunnel);
	return t(i18n)`Tunnel ${endpoint} adopted`;
}

/** Actions on a tunnel held by an external ssh process or blocked by another process. */
export function useExternalActions(
	tunnel: Readonly<TunnelDto>
): UseExternalActionsReturn {
	const { i18n } = useLingui();
	const runDaemonAction = useDaemonAction();
	const killProcess = useKillProcess();
	const ignoreProcess = useIgnoreProcess();
	const { state } = tunnel;

	const adopt = () => {
		void runDaemonAction(
			async (gateway) => {
				const adoptedIds = await gateway.adoptExternal(tunnel.id);
				useToastStore.getState().pushToast({
					tone: 'success',
					message: describeAdoption(tunnel, adoptedIds, i18n),
				});
			},
			t(i18n)`Couldn't adopt the tunnel`
		);
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
