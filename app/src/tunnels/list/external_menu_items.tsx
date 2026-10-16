import { MenuItem, MenuSeparator } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { useExternalActions } from '#/tunnels/list/use_external_actions';

type ExternalMenuItemsProps = {
	tunnel: TunnelDto;
};

/** Items for a tunnel held by an external ssh process or blocked by another process. */
export function ExternalMenuItems({
	tunnel,
}: Readonly<ExternalMenuItemsProps>) {
	const { adopt, confirmKill, ignore } = useExternalActions(tunnel);
	const { state } = tunnel;

	const handleKill = () => void confirmKill();
	const handleIgnore = () => void ignore();

	if (state.kind === 'external') {
		return (
			<>
				<MenuItem icon="i-mdi-download-circle-outline" onClick={adopt}>
					Adopter
				</MenuItem>
				<MenuItem
					icon="i-mdi-close-octagon-outline"
					danger
					onClick={handleKill}
				>
					Arrêter le processus externe…
				</MenuItem>
				<MenuItem icon="i-mdi-eye-off-outline" onClick={handleIgnore}>
					Ignorer
				</MenuItem>
				<MenuSeparator />
			</>
		);
	}
	if (state.kind === 'conflict' && state.owner) {
		return (
			<>
				<MenuItem
					icon="i-mdi-close-octagon-outline"
					danger
					onClick={handleKill}
				>
					Arrêter le processus…
				</MenuItem>
				<MenuSeparator />
			</>
		);
	}
	return null;
}
