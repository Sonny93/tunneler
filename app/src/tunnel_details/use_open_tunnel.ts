import type { TunnelDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import {
	describeOpenAction,
	listAlternativeChoices,
	toDefaultChoice,
	type OpenChoice,
} from '#/openers/open_choices';

type UseOpenTunnelReturn = {
	/** Only a local forward has something to open; the daemon starts it when stopped. */
	isOpenable: boolean;
	/** Says what the primary action does: the browser or the default application. */
	label: string;
	/** The other ways to open the tunnel: "Ouvrir avec…". */
	alternatives: readonly OpenChoice[];
	/** Opens with the default action. */
	open: () => void;
	openWith: (opener: string) => void;
};

export function useOpenTunnel(
	tunnel: Readonly<TunnelDto>
): UseOpenTunnelReturn {
	const runDaemonAction = useDaemonAction();
	const openers = useDaemonStore((state) => state.openers);

	const run = (opener?: string) => {
		void runDaemonAction(
			(gateway) => gateway.openTunnel(tunnel.id, opener),
			'Ouverture impossible'
		);
	};

	return {
		isOpenable: tunnel.kind === 'local',
		label: describeOpenAction(toDefaultChoice(tunnel.defaultOpener)),
		alternatives: listAlternativeChoices(tunnel.defaultOpener, openers),
		open: () => run(),
		openWith: run,
	};
}
