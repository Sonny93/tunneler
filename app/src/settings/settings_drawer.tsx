import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { SettingsPanel } from '#/settings/settings_panel';

/** The settings drawer, open while the UI panel is `settings`. */
export function SettingsDrawer() {
	const panel = useUiStore((state) => state.panel);
	const closePanel = useUiStore((state) => state.closePanel);
	const settings = useDaemonStore((state) => state.settings);

	if (panel.kind !== 'settings') return null;
	if (settings === null) {
		return (
			<Drawer isOpen title="Paramètres" onClose={closePanel}>
				<p>Paramètres indisponibles.</p>
			</Drawer>
		);
	}
	return <SettingsPanel settings={settings} />;
}
