import { useLingui } from '@lingui/react/macro';

import { ImportPanel } from '#/import/import_panel';
import { Drawer } from '#/shared/drawer';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';

/** The import drawer, open while the UI panel is `import`. */
export function ImportDrawer() {
	const { t } = useLingui();
	const panel = useUiStore((state) => state.panel);
	const closePanel = useUiStore((state) => state.closePanel);
	const settings = useDaemonStore((state) => state.settings);

	if (panel.kind !== 'import') return null;
	const title = t`Import tunnels`;
	if (settings === null) {
		return (
			<Drawer isOpen title={title} onClose={closePanel}>
				<p>{t`Import unavailable.`}</p>
			</Drawer>
		);
	}
	return <ImportPanel title={title} settings={settings} />;
}
