import { useCallback } from 'react';
import { useLingui } from '@lingui/react/macro';

import type { TunnelDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import { confirmStandby } from '#/standby/confirm_standby';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { listAwakeTunnels } from '#/standby/list_awake_tunnels';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

type UseSendToStandbyReturn = {
	sendTunnel: (tunnel: Readonly<TunnelDto>) => Promise<void>;
	sendTag: (tag: string) => Promise<void>;
	sendAll: () => Promise<void>;
};

/**
 * Sends awake tunnels back to standby, one tunnel, one tag or all of them.
 * Asks for confirmation first when open connections would be cut.
 */
export function useSendToStandby(): UseSendToStandbyReturn {
	const { t, i18n } = useLingui();
	const runDaemonAction = useDaemonAction();

	const confirmAndSend = useCallback(
		async (
			title: string,
			targets: readonly TunnelDto[],
			send: (gateway: DaemonGateway) => Promise<unknown>
		) => {
			if (!(await confirmStandby(title, targets, i18n))) return;
			await runDaemonAction(send, t`Couldn't send to standby`);
		},
		[i18n, runDaemonAction, t]
	);

	const sendTunnel = useCallback(
		(tunnel: Readonly<TunnelDto>) => {
			const endpoint = getTunnelEndpointLabel(tunnel);
			return confirmAndSend(
				t`Send tunnel ${endpoint} to standby?`,
				[tunnel],
				(gateway) => gateway.standbyTunnel(tunnel.id)
			);
		},
		[confirmAndSend, t]
	);

	const sendTag = useCallback(
		(tag: string) => {
			const targets = listAwakeTunnels(
				useDaemonStore
					.getState()
					.tunnels.filter((tunnel) => tunnel.tags.includes(tag))
			);
			return confirmAndSend(
				t`Send tag ${tag} to standby?`,
				targets,
				(gateway) => gateway.standbyTag(tag)
			);
		},
		[confirmAndSend, t]
	);

	const sendAll = useCallback(() => {
		const targets = listAwakeTunnels(useDaemonStore.getState().tunnels);
		return confirmAndSend(
			t`Send the awake tunnels to standby?`,
			targets,
			(gateway) => gateway.standbyAll()
		);
	}, [confirmAndSend, t]);

	return { sendTunnel, sendTag, sendAll };
}
