import { useEffect } from 'react';

import { useDaemonStore } from '#/state/daemon_store';
import { useUpdateStore } from '#/updates/update_store';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';

/**
 * Looks for a newer version once per UI session, when the connection is up
 * and the `checkForUpdates` setting is on. A failed check is logged by the
 * native side and shows nothing.
 */
export function useUpdateCheck(): void {
	const gateway = useDaemonGateway();
	const isConnected = useDaemonStore(
		(state) => state.connection.status === 'connected'
	);
	const hasSettings = useDaemonStore((state) => state.settings !== null);

	useEffect(() => {
		const { hasChecked, markChecked, setOffer } = useUpdateStore.getState();
		const settings = useDaemonStore.getState().settings;
		if (!isConnected || settings === null || hasChecked) return;
		markChecked();
		if (!settings.checkForUpdates) return;
		void gateway.checkForUpdate().then(setOffer, () => setOffer(null));
	}, [gateway, isConnected, hasSettings]);
}
