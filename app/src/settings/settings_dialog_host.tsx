import { useLingui } from '@lingui/react/macro';

import { SettingsDialogContent } from '#/settings/settings_dialog_content';
import { useModalCall } from '#/shared/use_modal_call';
import { useUiStore } from '#/state/ui_store';

const SETTINGS_MODAL_ID = 'settings';

/** Opens the settings dialog while the UI panel is `settings`; a `<ModalProvider />` must be mounted. */
export function SettingsDialogHost(): null {
	const { t } = useLingui();
	const panel = useUiStore((state) => state.panel);
	const closePanel = useUiStore((state) => state.closePanel);

	const request =
		panel.kind === 'settings'
			? {
					id: SETTINGS_MODAL_ID,
					props: {
						title: t`Settings`,
						size: 'xl' as const,
						height: 'fixed' as const,
						children: <SettingsDialogContent initialTab={panel.tab} />,
					},
				}
			: null;
	useModalCall(request, closePanel);

	return null;
}
