import { IconButton, Menu, MenuItem } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { useTunnelActions } from '#/tunnels/list/use_tunnel_actions';
import { ExternalMenuItems } from '#/tunnels/list/external_menu_items';
import { useOpenInBrowser } from '#/tunnel_details/use_open_in_browser';
import { ROVING_CONTROL_ATTRIBUTE } from '#/tunnels/list/use_roving_rows';

type TunnelActionsMenuProps = {
	tunnel: TunnelDto;
	isActiveRow: boolean;
};

export function TunnelActionsMenu({
	tunnel,
	isActiveRow,
}: Readonly<TunnelActionsMenuProps>) {
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const openTunnelDetails = useUiStore((state) => state.openTunnelDetails);
	const { restart, confirmRemoval } = useTunnelActions(tunnel);
	const { target, isRunning, open } = useOpenInBrowser(tunnel);

	const handleDetails = () => openTunnelDetails(tunnel.id);
	const handleEdit = () => openTunnelEditor(tunnel.id);
	const handleRemove = () => void confirmRemoval();

	const trigger = (
		<IconButton
			icon="i-mdi-dots-vertical"
			aria-label={`Actions pour ${tunnel.name}`}
			variant="ghost"
			color="neutral"
			size="sm"
			className="size-8"
			tabIndex={isActiveRow ? 0 : -1}
			{...{ [ROVING_CONTROL_ATTRIBUTE]: 'actions' }}
		/>
	);

	return (
		<Menu trigger={trigger} align="end">
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
		</Menu>
	);
}
