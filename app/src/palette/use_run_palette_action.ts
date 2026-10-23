import { useLingui } from '@lingui/react/macro';

import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useOpenDocumentation } from '#/documentation/use_open_documentation';
import type { PaletteAction } from '#/palette/palette_command';
import { useSendToStandby } from '#/standby/use_send_to_standby';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { useDeleteTag } from '#/tag_delete/use_delete_tag';

type RunPaletteAction = (action: PaletteAction) => void;

/** Runs a palette action with the same effects as the matching buttons and menu items. */
export function useRunPaletteAction(): RunPaletteAction {
	const { t } = useLingui();
	const runDaemonAction = useDaemonAction();
	const deleteTag = useDeleteTag();
	const { sendTunnel, sendTag, sendAll } = useSendToStandby();
	const openDocumentation = useOpenDocumentation();

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
					t`Couldn't open the folder`
				);
			case 'renameTag':
				return ui.openTagRename(action.tag, 'palette');
			case 'deleteTag':
				return void deleteTag(action.tag);
			case 'toggleSidebar':
				return ui.toggleSidebar();
			case 'showShortcuts':
				return ui.setHelpOpen(true);
			case 'openDocumentation':
				return void openDocumentation('home');
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
					t`Couldn't start the tunnel`
				);
			case 'stopTunnel':
				return void runDaemonAction(
					(gateway) => gateway.stopTunnel(action.tunnelId),
					t`Couldn't stop the tunnel`
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
					t`Couldn't start the tunnels`
				);
			case 'stopTag':
				return void runDaemonAction(
					(gateway) => gateway.stopTag(action.tag),
					t`Couldn't stop the tunnels`
				);
			case 'openTunnel':
				return void runDaemonAction(
					(gateway) => gateway.openTunnel(action.id, action.opener),
					t`Couldn't open the tunnel`
				);
		}
	};
}
