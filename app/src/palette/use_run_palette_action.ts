import { useUiStore } from '#/state/ui_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import type { PaletteAction } from '#/palette/palette_command';

type RunPaletteAction = (action: PaletteAction) => void;

/** Runs a palette action with the same effects as the matching buttons and menu items. */
export function useRunPaletteAction(): RunPaletteAction {
	const runDaemonAction = useDaemonAction();

	return (action) => {
		const ui = useUiStore.getState();
		switch (action.kind) {
			case 'newTunnel':
				return ui.openTunnelEditor(null);
			case 'importTunnels':
				return ui.openImport();
			case 'openSettings':
				return ui.openSettings();
			case 'revealConfig':
				return void runDaemonAction(
					(gateway) => gateway.revealConfig(),
					'Ouverture impossible'
				);
			case 'renameTag':
				return ui.openTagRename(action.tag, 'palette');
			case 'toggleSidebar':
				return ui.toggleSidebar();
			case 'showShortcuts':
				return ui.setHelpOpen(true);
			case 'showTunnelDetails':
				return ui.openTunnelDetails(action.tunnelId);
			case 'editTunnel':
				return ui.openTunnelEditor(action.tunnelId);
			case 'showHostConfig':
				return ui.openHostConfig(action.host);
			case 'showX11':
				return ui.openX11(action.host);
			case 'startTunnel':
				return void runDaemonAction(
					(gateway) => gateway.startTunnel(action.tunnelId),
					'Démarrage impossible'
				);
			case 'stopTunnel':
				return void runDaemonAction(
					(gateway) => gateway.stopTunnel(action.tunnelId),
					'Arrêt impossible'
				);
			case 'startTag':
				return void runDaemonAction(
					(gateway) => gateway.startTag(action.tag),
					'Démarrage impossible'
				);
			case 'stopTag':
				return void runDaemonAction(
					(gateway) => gateway.stopTag(action.tag),
					'Arrêt impossible'
				);
			case 'openInBrowser':
				return void runDaemonAction(
					(gateway) => gateway.openInBrowser(action.host, action.port),
					'Ouverture impossible'
				);
		}
	};
}
