import { useCallback } from 'react';

import type { TunnelDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import { confirmStandby } from '#/standby/confirm_standby';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { listAwakeTunnels } from '#/standby/list_awake_tunnels';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

const FAILURE_MESSAGE = 'Mise en veille impossible';

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
	const runDaemonAction = useDaemonAction();

	const confirmAndSend = useCallback(
		async (
			title: string,
			targets: readonly TunnelDto[],
			send: (gateway: DaemonGateway) => Promise<unknown>
		) => {
			if (!(await confirmStandby(title, targets))) return;
			await runDaemonAction(send, FAILURE_MESSAGE);
		},
		[runDaemonAction]
	);

	const sendTunnel = useCallback(
		(tunnel: Readonly<TunnelDto>) =>
			confirmAndSend(
				`Mettre en veille le tunnel ${getTunnelEndpointLabel(tunnel)} ?`,
				[tunnel],
				(gateway) => gateway.standbyTunnel(tunnel.id)
			),
		[confirmAndSend]
	);

	const sendTag = useCallback(
		(tag: string) => {
			const targets = listAwakeTunnels(
				useDaemonStore
					.getState()
					.tunnels.filter((tunnel) => tunnel.tags.includes(tag))
			);
			return confirmAndSend(
				`Mettre en veille le tag ${tag} ?`,
				targets,
				(gateway) => gateway.standbyTag(tag)
			);
		},
		[confirmAndSend]
	);

	const sendAll = useCallback(() => {
		const targets = listAwakeTunnels(useDaemonStore.getState().tunnels);
		return confirmAndSend(
			'Mettre en veille les tunnels réveillés ?',
			targets,
			(gateway) => gateway.standbyAll()
		);
	}, [confirmAndSend]);

	return { sendTunnel, sendTag, sendAll };
}
