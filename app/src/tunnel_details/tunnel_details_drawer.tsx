import { useEffect } from 'react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { TunnelDetailsPanel } from '#/tunnel_details/tunnel_details_panel';

type TunnelDetailsProps = {
	tunnelId: string;
};

function TunnelDetails({ tunnelId }: Readonly<TunnelDetailsProps>) {
	const tunnel = useDaemonStore((state) =>
		state.tunnels.find((candidate) => candidate.id === tunnelId)
	);
	const closePanel = useUiStore((state) => state.closePanel);
	const isMissing = tunnel === undefined;

	useEffect(() => {
		if (isMissing) closePanel();
	}, [isMissing, closePanel]);

	if (isMissing) return null;
	return <TunnelDetailsPanel tunnel={tunnel} />;
}

/** The details drawer, open while the UI panel is `tunnelDetails`. */
export function TunnelDetailsDrawer() {
	const panel = useUiStore((state) => state.panel);
	if (panel.kind !== 'tunnelDetails') return null;

	return <TunnelDetails key={panel.tunnelId} tunnelId={panel.tunnelId} />;
}
