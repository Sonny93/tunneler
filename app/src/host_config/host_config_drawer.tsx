import { HostConfigPanel } from '#/host_config/host_config_panel';
import { useRetainedPanel } from '#/shared/use_retained_panel';
import { useUiStore } from '#/state/ui_store';

/** The resolved host configuration drawer, open while the UI panel is `hostConfig` and kept mounted while it slides out. */
export function HostConfigDrawer() {
	const panel = useUiStore((state) => state.panel);
	const {
		panel: hostConfigPanel,
		isOpen,
		instanceKey,
	} = useRetainedPanel(panel.kind === 'hostConfig' ? panel : null);

	if (hostConfigPanel === null) return null;
	return (
		<HostConfigPanel
			key={instanceKey}
			host={hostConfigPanel.host}
			isOpen={isOpen}
		/>
	);
}
