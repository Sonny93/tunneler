import { useEffect } from 'react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { TunnelFormPanel } from '#/tunnel_form/tunnel_form_panel';

const NEW_TUNNEL_KEY = 'new';

type TunnelEditorProps = {
	tunnelId: string | null;
};

function TunnelEditor({ tunnelId }: Readonly<TunnelEditorProps>) {
	const tunnel = useDaemonStore((state) =>
		state.tunnels.find((candidate) => candidate.id === tunnelId)
	);
	const closePanel = useUiStore((state) => state.closePanel);
	const isMissing = tunnelId !== null && tunnel === undefined;

	useEffect(() => {
		if (isMissing) closePanel();
	}, [isMissing, closePanel]);

	if (isMissing) return null;
	return <TunnelFormPanel tunnel={tunnel ?? null} />;
}

/** The create/edit drawer, open while the UI panel is the tunnel editor. */
export function TunnelFormDrawer() {
	const panel = useUiStore((state) => state.panel);
	if (panel.kind !== 'tunnelEditor') return null;

	return (
		<TunnelEditor
			key={panel.tunnelId ?? NEW_TUNNEL_KEY}
			tunnelId={panel.tunnelId}
		/>
	);
}
