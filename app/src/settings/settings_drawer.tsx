import { useLingui } from '@lingui/react/macro';

import { SettingsPanel } from '#/settings/settings_panel';
import { Drawer } from '#/shared/drawer';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';

/** The settings drawer, open while the UI panel is `settings`. */
export function SettingsDrawer() {
	const panel = useUiStore((state) => state.panel);
	const closePanel = useUiStore((state) => state.closePanel);
	const settings = useDaemonStore((state) => state.settings);
	const { t } = useLingui();

	if (panel.kind !== 'settings') return null;
	if (settings === null) {
		return (
			<Drawer isOpen title={t`Settings`} onClose={closePanel}>
				<p>{t`Settings unavailable.`}</p>
			</Drawer>
		);
	}
	return <SettingsPanel settings={settings} />;
}
