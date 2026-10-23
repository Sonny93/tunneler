import { useLingui } from '@lingui/react/macro';
import { Drawer } from '@minimalstuff/ui';

import { SettingsPanel } from '#/settings/settings_panel';
import { useRetainedPanel } from '#/shared/use_retained_panel';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';

/** The settings drawer, open while the UI panel is `settings` and kept mounted while it slides out. */
export function SettingsDrawer() {
	const panel = useUiStore((state) => state.panel);
	const closePanel = useUiStore((state) => state.closePanel);
	const settings = useDaemonStore((state) => state.settings);
	const { t } = useLingui();
	const {
		panel: settingsPanel,
		isOpen,
		instanceKey,
	} = useRetainedPanel(panel.kind === 'settings' ? panel : null);

	if (settingsPanel === null) return null;
	if (settings === null) {
		return (
			<Drawer isOpen={isOpen} title={t`Settings`} onClose={closePanel}>
				<p>{t`Settings unavailable.`}</p>
			</Drawer>
		);
	}
	return (
		<SettingsPanel key={instanceKey} settings={settings} isOpen={isOpen} />
	);
}
