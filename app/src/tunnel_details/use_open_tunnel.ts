import type { TunnelDto } from '#/daemon/protocol';
import { useDaemonAction } from '#/daemon/use_daemon_action';

type UseOpenTunnelReturn = {
	/** Only a local forward has something to browse; the daemon starts it when stopped. */
	isOpenable: boolean;
	open: () => void;
};

export function useOpenTunnel(
	tunnel: Readonly<TunnelDto>
): UseOpenTunnelReturn {
	const runDaemonAction = useDaemonAction();

	const open = () => {
		void runDaemonAction(
			(gateway) => gateway.openTunnel(tunnel.id),
			'Ouverture impossible'
		);
	};

	return { isOpenable: tunnel.kind === 'local', open };
}
