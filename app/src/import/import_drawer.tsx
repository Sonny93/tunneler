import { useLingui } from '@lingui/react/macro';
import { Drawer } from '@minimalstuff/ui';

import { ImportPanel } from '#/import/import_panel';
import { useRetainedPanel } from '#/shared/use_retained_panel';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';

/** The import drawer, open while the UI panel is `import` and kept mounted while it slides out. */
export function ImportDrawer() {
	const { t } = useLingui();
	const panel = useUiStore((state) => state.panel);
	const closePanel = useUiStore((state) => state.closePanel);
	const settings = useDaemonStore((state) => state.settings);
	const {
		panel: importPanel,
		isOpen,
		instanceKey,
	} = useRetainedPanel(panel.kind === 'import' ? panel : null);

	if (importPanel === null) return null;
	const title = t`Import tunnels`;
	if (settings === null) {
		return (
			<Drawer isOpen={isOpen} title={title} onClose={closePanel}>
				<p>{t`Import unavailable.`}</p>
			</Drawer>
		);
	}
	return (
		<ImportPanel
			key={instanceKey}
			title={title}
			settings={settings}
			isOpen={isOpen}
		/>
	);
}
