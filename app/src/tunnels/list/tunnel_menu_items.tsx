import { MenuItem } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { useOpenTunnel } from '#/tunnel_details/use_open_tunnel';
import { useTunnelActions } from '#/tunnels/list/use_tunnel_actions';
import { ExternalMenuItems } from '#/tunnels/list/external_menu_items';

type TunnelMenuItemsProps = {
	tunnel: TunnelDto;
};

/** The actions on one tunnel, shared by its kebab `Menu` and its right-click `ContextMenu`. */
export function TunnelMenuItems({ tunnel }: Readonly<TunnelMenuItemsProps>) {
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const openTunnelDetails = useUiStore((state) => state.openTunnelDetails);
	const { restart, stopAndRemove, persist, confirmRemoval } =
		useTunnelActions(tunnel);
	const { isOpenable, open } = useOpenTunnel(tunnel);
	const isRunning = isTunnelRunning(tunnel.state);

	const handleDetails = () => openTunnelDetails(tunnel.id);
	const handleEdit = () => openTunnelEditor(tunnel.id);
	const handleRemove = () => void confirmRemoval();

	return (
		<>
			<MenuItem icon="i-mdi-information-outline" onClick={handleDetails}>
				Détails
			</MenuItem>
			{isOpenable && (
				<MenuItem icon="i-mdi-open-in-new" onClick={open}>
					Ouvrir dans le navigateur
				</MenuItem>
			)}
			<ExternalMenuItems tunnel={tunnel} />
			{tunnel.isTemporary ? (
				<MenuItem icon="i-mdi-content-save-outline" onClick={persist}>
					Enregistrer le tunnel
				</MenuItem>
			) : (
				<MenuItem icon="i-mdi-pencil-outline" onClick={handleEdit}>
					Modifier
				</MenuItem>
			)}
			<MenuItem icon="i-mdi-restart" disabled={!isRunning} onClick={restart}>
				Redémarrer
			</MenuItem>
			{tunnel.isTemporary ? (
				<MenuItem icon="i-mdi-stop" danger onClick={stopAndRemove}>
					Arrêter et retirer
				</MenuItem>
			) : (
				<MenuItem icon="i-mdi-delete-outline" danger onClick={handleRemove}>
					Supprimer
				</MenuItem>
			)}
		</>
	);
}
