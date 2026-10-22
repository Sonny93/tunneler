import { ConfirmModal } from '@minimalstuff/ui';

import type { TunnelDto } from '#/daemon/protocol';
import { countOpenConnections } from '#/standby/count_open_connections';
import { describeCutConnections } from '#/standby/describe_cut_connections';

/**
 * Asks the user to confirm sending `tunnels` back to standby when that cuts
 * open connections; resolves to whether to go on (at once when none is open).
 */
export async function confirmStandby(
	title: string,
	tunnels: readonly Readonly<TunnelDto>[]
): Promise<boolean> {
	const openConnectionCount = countOpenConnections(tunnels);
	if (openConnectionCount === 0) return true;
	return ConfirmModal.call({
		title,
		children: describeCutConnections(openConnectionCount),
		confirmLabel: 'Mettre en veille',
	});
}
