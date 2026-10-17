import type { TunnelDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import {
	getBrowserTarget,
	type BrowserTarget,
} from '#/tunnel_details/browser_target';

type UseOpenInBrowserReturn = {
	target: BrowserTarget | null;
	isRunning: boolean;
	open: () => void;
};

/** What opening a local forward in the browser needs: its URL target and whether it is reachable. */
export function useOpenInBrowser(
	tunnel: Readonly<TunnelDto>
): UseOpenInBrowserReturn {
	const runDaemonAction = useDaemonAction();
	const defaultBindAddress = useDaemonStore(
		(state) => state.settings?.defaultBindAddress
	);
	const target = getBrowserTarget(tunnel, defaultBindAddress);

	const open = () => {
		if (target === null) return;
		void runDaemonAction(
			(gateway) => gateway.openInBrowser(target.host, target.port),
			'Ouverture impossible'
		);
	};

	return { target, isRunning: isTunnelRunning(tunnel.state), open };
}
