import { useContext } from 'react';
import { useLingui } from '@lingui/react/macro';
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
	const { t } = useLingui();
	const runDaemonAction = useDaemonAction();
	const requestFocusAfterRemoval = useContext(RemovalFocusContext);
	const { id } = tunnel;
	const label = getTunnelEndpointLabel(tunnel);

	const stopAndRemove = () => {
		void (async () => {
			const isStopped = await runDaemonAction(
				(gateway) => gateway.stopTunnel(id),
				t`Couldn't stop the tunnel`
			);
			if (!isStopped) return;
			useAnnouncerStore
				.getState()
				.announce(t`Temporary tunnel ${label} stopped and removed`, 'polite');
			requestFocusAfterRemoval([id]);
		})();
	};

	const persist = () => {
		void (async () => {
			const isSaved = await runDaemonAction(
				(gateway) => gateway.persistTunnel(id),
				t`Couldn't save the tunnel`
			);
			if (!isSaved) return;
			useToastStore
				.getState()
				.pushToast({ tone: 'success', message: t`Tunnel ${label} saved` });
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
				t`Couldn't stop the tunnel`
			);
			return;
		}
		void runDaemonAction(
			(gateway) => gateway.startTunnel(id),
			t`Couldn't start the tunnel`
		);
	};

	const restart = () => {
		void runDaemonAction(
			(gateway) => gateway.restartTunnel(id),
			t`Couldn't restart the tunnel`
		);
	};

	const confirmRemoval = async () => {
		const isConfirmed = await ConfirmModal.call({
			title: t`Delete tunnel ${label}?`,
			children: t`The tunnel is removed from the configuration, and stopped if it is running.`,
			confirmLabel: t`Delete`,
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		const isDeleted = await runDaemonAction(
			(gateway) => gateway.deleteTunnel(id),
			t`Couldn't delete the tunnel`
		);
		if (!isDeleted) return;
		useToastStore
			.getState()
			.pushToast({ tone: 'success', message: t`Tunnel ${label} deleted` });
		requestFocusAfterRemoval([id]);
	};

	return { toggleRunning, restart, stopAndRemove, persist, confirmRemoval };
}
