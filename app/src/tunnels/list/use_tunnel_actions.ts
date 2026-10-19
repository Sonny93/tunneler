import { useContext } from 'react';
import { ConfirmModal } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { RemovalFocusContext } from '#/tunnels/list/removal_focus_context';

type UseTunnelActionsReturn = {
	toggleRunning: () => void;
	restart: () => void;
	confirmRemoval: () => Promise<void>;
};

export function useTunnelActions(
	tunnel: Readonly<TunnelDto>
): UseTunnelActionsReturn {
	const runDaemonAction = useDaemonAction();
	const requestFocusAfterRemoval = useContext(RemovalFocusContext);
	const { id, name } = tunnel;

	const toggleRunning = () => {
		if (isTunnelRunning(tunnel.state)) {
			void runDaemonAction(
				(gateway) => gateway.stopTunnel(id),
				'Arrêt impossible'
			);
			return;
		}
		void runDaemonAction(
			(gateway) => gateway.startTunnel(id),
			'Démarrage impossible'
		);
	};

	const restart = () => {
		void runDaemonAction(
			(gateway) => gateway.restartTunnel(id),
			'Redémarrage impossible'
		);
	};

	const confirmRemoval = async () => {
		const isConfirmed = await ConfirmModal.call({
			title: `Supprimer le tunnel ${name} ?`,
			children:
				'Le tunnel est retiré de la configuration, et arrêté s’il est en cours.',
			confirmLabel: 'Supprimer',
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		const isDeleted = await runDaemonAction(
			(gateway) => gateway.deleteTunnel(id),
			'Suppression impossible'
		);
		if (!isDeleted) return;
		useToastStore
			.getState()
			.pushToast({ tone: 'success', message: `Tunnel ${name} supprimé` });
		requestFocusAfterRemoval([id]);
	};

	return { toggleRunning, restart, confirmRemoval };
}
