import { useEffect } from 'react';

import { useQuitFlow } from '#/quit/use_quit_flow';
import { useDaemonStore } from '#/state/daemon_store';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';

/** Runs the quit flow when another part of the app (the tray) requests a quit. */
export function useQuitRequestListener(): void {
	const isQuitRequested = useDaemonStore((state) => state.isQuitRequested);
	const { quit, confirmAndQuit } = useQuitFlow();

	useEffect(() => {
		if (!isQuitRequested) return;
		const { tunnels, clearQuitRequest } = useDaemonStore.getState();
		if (!useDaemonStore.getState().isQuitRequested) return;
		clearQuitRequest();
		const runningCount = tunnels.filter((tunnel) =>
			isTunnelRunning(tunnel.state)
		).length;
		if (runningCount === 0) {
			void quit();
			return;
		}
		void confirmAndQuit(runningCount);
	}, [isQuitRequested, quit, confirmAndQuit]);
}
