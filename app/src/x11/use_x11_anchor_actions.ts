import { ConfirmModal } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';

type UseX11AnchorActionsReturn = {
	start: () => Promise<void>;
	stop: () => Promise<void>;
};

async function confirmRisk(host: string): Promise<boolean> {
	const { confirmedX11Hosts, confirmX11Host } = useUiStore.getState();
	if (confirmedX11Hosts.includes(host)) return true;
	const isConfirmed = await ConfirmModal.call({
		title: `Activer X11 pour ${host} ?`,
		children: `Le serveur ${host} aura un accès complet à votre affichage local (capture du clavier et de l'écran possible). N'activez X11 que pour un hôte de confiance.`,
		confirmLabel: 'Activer X11',
		confirmColor: 'danger',
	});
	if (isConfirmed) confirmX11Host(host);
	return isConfirmed;
}

/** Starting asks for confirmation the first time per host in the UI session; stopping never does. */
export function useX11AnchorActions(host: string): UseX11AnchorActionsReturn {
	const runDaemonAction = useDaemonAction();

	const start = async () => {
		if (!(await confirmRisk(host))) return;
		await runDaemonAction(
			(gateway) => gateway.startX11(host),
			`Démarrage de X11 pour ${host} impossible`
		);
	};
	const stop = async () => {
		await runDaemonAction(
			(gateway) => gateway.stopX11(host),
			`Arrêt de X11 pour ${host} impossible`
		);
	};

	return { start, stop };
}
