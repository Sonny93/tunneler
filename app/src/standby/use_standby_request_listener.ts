import { useEffect } from 'react';

import { useSendToStandby } from '#/standby/use_send_to_standby';
import { useDaemonStore } from '#/state/daemon_store';

/** Sends the awake tunnels back to standby when another part of the app (the tray) requests it. */
export function useStandbyRequestListener(): void {
	const isStandbyRequested = useDaemonStore(
		(state) => state.isStandbyRequested
	);
	const { sendAll } = useSendToStandby();

	useEffect(() => {
		if (!isStandbyRequested) return;
		useDaemonStore.getState().clearStandbyRequest();
		void sendAll();
	}, [isStandbyRequested, sendAll]);
}
