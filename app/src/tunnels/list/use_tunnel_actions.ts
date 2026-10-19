import { useContext } from 'react';
import { ConfirmModal } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';
import { RemovalFocusContext } from '#/tunnels/list/removal_focus_context';

type UseTunnelActionsReturn = {
	toggleRunning: () => void;
	restart: () => void;
	/** Stops a temporary tunnel, which removes it. */
	stopAndRemove: () => void;
	/** Saves a temporary tunnel; it keeps running. */
	persist: () => void;
	confirmRemoval: () => Promise<void>;
};

export function useTunnelActions(
	tunnel: Readonly<TunnelDto>
): UseTunnelActionsReturn {
	const runDaemonAction = useDaemonAction();
	const requestFocusAfterRemoval = useContext(RemovalFocusContext);
	const { id } = tunnel;
	const label = getTunnelEndpointLabel(tunnel);

	const stopAndRemove = () => {
		void (async () => {
			const isStopped = await runDaemonAction(
				(gateway) => gateway.stopTunnel(id),
				'Arrêt impossible'
			);
			if (!isStopped) return;
			useAnnouncerStore
				.getState()
				.announce(`Tunnel temporaire ${label} arrêté et retiré`, 'polite');
			requestFocusAfterRemoval([id]);
		})();
	};

	const persist = () => {
		void (async () => {
			const isSaved = await runDaemonAction(
				(gateway) => gateway.persistTunnel(id),
				'Enregistrement impossible'
			);
			if (!isSaved) return;
			useToastStore
				.getState()
				.pushToast({ tone: 'success', message: `Tunnel ${label} enregistré` });
		})();
	};

	const toggleRunning = () => {
		const isRunning = isTunnelRunning(tunnel.state);
		if (isRunning && tunnel.isTemporary) {
			stopAndRemove();
			return;
		}
		if (isRunning) {
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
			title: `Supprimer le tunnel ${label} ?`,
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
			.pushToast({ tone: 'success', message: `Tunnel ${label} supprimé` });
		requestFocusAfterRemoval([id]);
	};

	return { toggleRunning, restart, stopAndRemove, persist, confirmRemoval };
}
