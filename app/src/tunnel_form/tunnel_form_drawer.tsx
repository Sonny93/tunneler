import { useEffect } from 'react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import type { TunnelSpecDto } from '#/daemon/protocol';
import { TunnelFormPanel } from '#/tunnel_form/tunnel_form_panel';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

const NEW_TUNNEL_KEY = 'new';

type TunnelEditorProps = {
	tunnelId: string | null;
	draft?: TunnelSpecDto;
};

function TunnelEditor({ tunnelId, draft }: Readonly<TunnelEditorProps>) {
	const tunnel = useDaemonStore((state) =>
		state.tunnels.find((candidate) => candidate.id === tunnelId)
	);
	const closePanel = useUiStore((state) => state.closePanel);
	const isMissing = tunnelId !== null && tunnel === undefined;

	useEffect(() => {
		if (isMissing) closePanel();
	}, [isMissing, closePanel]);

	if (isMissing) return null;
	return <TunnelFormPanel tunnel={tunnel ?? null} draft={draft} />;
}

/** The create/edit drawer, open while the UI panel is the tunnel editor. */
export function TunnelFormDrawer() {
	const panel = useUiStore((state) => state.panel);
	if (panel.kind !== 'tunnelEditor') return null;

	return (
		<TunnelEditor
			key={
				panel.tunnelId ??
				(panel.draft ? getTunnelEndpointLabel(panel.draft) : NEW_TUNNEL_KEY)
			}
			tunnelId={panel.tunnelId}
			draft={panel.draft}
		/>
	);
}
