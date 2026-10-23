import { useEffect } from 'react';

import type { TunnelSpecDto } from '#/daemon/protocol';
import { useRetainedPanel } from '#/shared/use_retained_panel';
import { useRetainedValue } from '#/shared/use_retained_value';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { TunnelFormPanel } from '#/tunnel_form/tunnel_form_panel';

type TunnelEditorProps = {
	tunnelId: string | null;
	draft?: TunnelSpecDto;
	isOpen: boolean;
};

function TunnelEditor({
	tunnelId,
	draft,
	isOpen,
}: Readonly<TunnelEditorProps>) {
	const liveTunnel = useDaemonStore((state) =>
		state.tunnels.find((candidate) => candidate.id === tunnelId)
	);
	const tunnel = useRetainedValue(liveTunnel ?? null);
	const closePanel = useUiStore((state) => state.closePanel);
	const isMissing = tunnelId !== null && liveTunnel === undefined;

	useEffect(() => {
		if (isOpen && isMissing) closePanel();
	}, [isOpen, isMissing, closePanel]);

	if (tunnelId !== null && tunnel === null) return null;
	return (
		<TunnelFormPanel
			tunnel={tunnel}
			draft={draft}
			isOpen={isOpen && !isMissing}
		/>
	);
}

/** The create/edit drawer, open while the UI panel is the tunnel editor and kept mounted while it slides out. */
export function TunnelFormDrawer() {
	const panel = useUiStore((state) => state.panel);
	const isEditorPanel =
		panel.kind === 'tunnelEditor' || panel.kind === 'temporaryTunnelEditor';
	const {
		panel: editorPanel,
		isOpen,
		instanceKey,
	} = useRetainedPanel(isEditorPanel ? panel : null);

	if (editorPanel === null) return null;
	if (editorPanel.kind === 'temporaryTunnelEditor') {
		return (
			<TunnelFormPanel
				key={instanceKey}
				tunnel={null}
				mode="temporary"
				isOpen={isOpen}
			/>
		);
	}

	return (
		<TunnelEditor
			key={instanceKey}
			tunnelId={editorPanel.tunnelId}
			draft={editorPanel.draft}
			isOpen={isOpen}
		/>
	);
}
