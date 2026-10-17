import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import { ImportPanel } from '#/import/import_panel';
import { useDaemonStore } from '#/state/daemon_store';

const TITLE = 'Importer des tunnels';

/** The import drawer, open while the UI panel is `import`. */
export function ImportDrawer() {
	const panel = useUiStore((state) => state.panel);
	const closePanel = useUiStore((state) => state.closePanel);
	const settings = useDaemonStore((state) => state.settings);

	if (panel.kind !== 'import') return null;
	if (settings === null) {
		return (
			<Drawer isOpen title={TITLE} onClose={closePanel}>
				<p>Import indisponible.</p>
			</Drawer>
		);
	}
	return <ImportPanel title={TITLE} settings={settings} />;
}
