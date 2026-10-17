import { useUiStore } from '#/state/ui_store';
import { HostConfigPanel } from '#/host_config/host_config_panel';

/** The resolved host configuration drawer, open while the UI panel is `hostConfig`. */
export function HostConfigDrawer() {
	const panel = useUiStore((state) => state.panel);

	if (panel.kind !== 'hostConfig') return null;
	return <HostConfigPanel key={panel.host} host={panel.host} />;
}
