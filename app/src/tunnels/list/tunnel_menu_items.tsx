import { MenuItem } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { useTunnelActions } from '#/tunnels/list/use_tunnel_actions';
import { ExternalMenuItems } from '#/tunnels/list/external_menu_items';
import { useOpenInBrowser } from '#/tunnel_details/use_open_in_browser';

type TunnelMenuItemsProps = {
	tunnel: TunnelDto;
};

/** The actions on one tunnel, shared by its kebab `Menu` and its right-click `ContextMenu`. */
export function TunnelMenuItems({ tunnel }: Readonly<TunnelMenuItemsProps>) {
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const openTunnelDetails = useUiStore((state) => state.openTunnelDetails);
	const { restart, confirmRemoval } = useTunnelActions(tunnel);
	const { target, isRunning, open } = useOpenInBrowser(tunnel);

	const handleDetails = () => openTunnelDetails(tunnel.id);
	const handleEdit = () => openTunnelEditor(tunnel.id);
	const handleRemove = () => void confirmRemoval();

	return (
		<>
			<MenuItem icon="i-mdi-information-outline" onClick={handleDetails}>
				Détails
			</MenuItem>
			{target !== null && (
				<MenuItem icon="i-mdi-open-in-new" disabled={!isRunning} onClick={open}>
					Ouvrir dans le navigateur
				</MenuItem>
			)}
			<ExternalMenuItems tunnel={tunnel} />
			<MenuItem icon="i-mdi-pencil-outline" onClick={handleEdit}>
				Modifier
			</MenuItem>
			<MenuItem icon="i-mdi-restart" disabled={!isRunning} onClick={restart}>
				Redémarrer
			</MenuItem>
			<MenuItem icon="i-mdi-delete-outline" danger onClick={handleRemove}>
				Supprimer
			</MenuItem>
		</>
	);
}
