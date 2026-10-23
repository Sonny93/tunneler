import { useLingui } from '@lingui/react/macro';
import { Input } from '@minimalstuff/ui';
import type { ChangeEvent } from 'react';

import type {
	SettingsTextField,
	SettingsToggleField,
} from '#/settings/settings_field_specs';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import { SettingsSwitch } from '#/settings/settings_switch';
import { summarizeTags } from '#/sidebar/summarize_tags';
import { useDaemonStore } from '#/state/daemon_store';

type SettingsStartupFieldsetProps = {
	values: SettingsFormValues;
	onChangeText: (field: SettingsTextField, value: string) => void;
	onChangeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
};

export function SettingsStartupFieldset({
	values,
	onChangeText,
	onChangeToggle,
}: Readonly<SettingsStartupFieldsetProps>) {
	const { t } = useLingui();
	const tunnels = useDaemonStore((state) => state.tunnels);
	const existingTags = summarizeTags(tunnels).map(({ tag }) => tag);
	const existingTagsText = existingTags.join(', ');
	const tagsHint =
		existingTags.length === 0
			? t`Separated by commas.`
			: t`Separated by commas. Existing tags: ${existingTagsText}.`;

	const handleTagsChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChangeText('startOnLaunchTags', event.target.value);

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">{t`Startup`}</legend>
			<SettingsSwitch
				field="launchAtLogin"
				label={t`Launch Tunneler at login`}
				hint={t`Starts the daemon and its tray icon without opening the window.`}
				isChecked={values.launchAtLogin}
				onChange={onChangeToggle}
			/>
			<SettingsSwitch
				field="restoreOnLaunch"
				label={t`Restore active tunnels on launch`}
				hint={t`Restarts the tunnels that were running when Tunneler last closed.`}
				isChecked={values.restoreOnLaunch}
				onChange={onChangeToggle}
			/>
			<Input
				name="startOnLaunchTags"
				label={t`Tags to start on launch`}
				description={tagsHint}
				value={values.startOnLaunchTags}
				onChange={handleTagsChange}
				autoComplete="off"
			/>
		</fieldset>
	);
}
