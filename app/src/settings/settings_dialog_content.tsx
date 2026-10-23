import { useLingui } from '@lingui/react/macro';

import { SettingsDialogForm } from '#/settings/settings_dialog_form';
import type { SettingsTab } from '#/settings/settings_tab';
import { useDaemonStore } from '#/state/daemon_store';

type SettingsDialogContentProps = {
	initialTab?: SettingsTab;
};

export function SettingsDialogContent({
	initialTab,
}: Readonly<SettingsDialogContentProps>) {
	const { t } = useLingui();
	const settings = useDaemonStore((state) => state.settings);

	if (settings === null) return <p>{t`Settings unavailable.`}</p>;
	return <SettingsDialogForm settings={settings} initialTab={initialTab} />;
}
