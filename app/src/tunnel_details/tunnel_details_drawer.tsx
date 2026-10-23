import { useEffect } from 'react';

import { useRetainedPanel } from '#/shared/use_retained_panel';
import { useRetainedValue } from '#/shared/use_retained_value';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { TunnelDetailsPanel } from '#/tunnel_details/tunnel_details_panel';

type TunnelDetailsProps = {
	tunnelId: string;
	isOpen: boolean;
};

function TunnelDetails({ tunnelId, isOpen }: Readonly<TunnelDetailsProps>) {
	const liveTunnel = useDaemonStore((state) =>
		state.tunnels.find((candidate) => candidate.id === tunnelId)
	);
	const tunnel = useRetainedValue(liveTunnel ?? null);
	const closePanel = useUiStore((state) => state.closePanel);
	const isMissing = liveTunnel === undefined;

	useEffect(() => {
		if (isOpen && isMissing) closePanel();
	}, [isOpen, isMissing, closePanel]);

	if (tunnel === null) return null;
	return <TunnelDetailsPanel tunnel={tunnel} isOpen={isOpen && !isMissing} />;
}

/** The details drawer, open while the UI panel is `tunnelDetails` and kept mounted while it slides out. */
export function TunnelDetailsDrawer() {
	const panel = useUiStore((state) => state.panel);
	const {
		panel: detailsPanel,
		isOpen,
		instanceKey,
	} = useRetainedPanel(panel.kind === 'tunnelDetails' ? panel : null);

	if (detailsPanel === null) return null;
	return (
		<TunnelDetails
			key={instanceKey}
			tunnelId={detailsPanel.tunnelId}
			isOpen={isOpen}
		/>
	);
}
