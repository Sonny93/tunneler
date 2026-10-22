import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { useDeleteTag } from '#/tag_delete/use_delete_tag';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import type { PaletteAction } from '#/palette/palette_command';
import { useSendToStandby } from '#/standby/use_send_to_standby';

type RunPaletteAction = (action: PaletteAction) => void;

/** Runs a palette action with the same effects as the matching buttons and menu items. */
export function useRunPaletteAction(): RunPaletteAction {
	const runDaemonAction = useDaemonAction();
	const deleteTag = useDeleteTag();
	const { sendTunnel, sendTag, sendAll } = useSendToStandby();

	const sendStandbyTunnel = async (tunnelId: string) => {
		const tunnel = useDaemonStore
			.getState()
			.tunnels.find(({ id }) => id === tunnelId);
		if (tunnel === undefined) return;
		await sendTunnel(tunnel);
	};

	return (action) => {
		const ui = useUiStore.getState();
		switch (action.kind) {
			case 'newTunnel':
				return ui.openTunnelEditor(null);
			case 'newTemporaryTunnel':
				return ui.openTemporaryTunnelEditor();
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
			case 'deleteTag':
				return void deleteTag(action.tag);
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
			case 'standbyTunnel':
				return void sendStandbyTunnel(action.tunnelId);
			case 'standbyTag':
				return void sendTag(action.tag);
			case 'standbyAll':
				return void sendAll();
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
			case 'openTunnel':
				return void runDaemonAction(
					(gateway) => gateway.openTunnel(action.id, action.opener),
					'Ouverture impossible'
				);
		}
	};
}
