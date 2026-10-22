import { useLingui } from '@lingui/react/macro';

import { SettingsSwitch } from '#/settings/settings_switch';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import type { SettingsToggleField } from '#/settings/settings_field_specs';

type SettingsUpdatesFieldsetProps = {
	values: SettingsFormValues;
	onChangeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
};

export function SettingsUpdatesFieldset({
	values,
	onChangeToggle,
}: Readonly<SettingsUpdatesFieldsetProps>) {
	const { t } = useLingui();

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">{t`Updates`}</legend>
			<SettingsSwitch
				field="checkForUpdates"
				label={t`Check for updates when opening`}
				hint={t`Each time the window opens, Tunneler contacts GitHub to see whether a new version exists. Nothing is installed without your approval.`}
				isChecked={values.checkForUpdates}
				onChange={onChangeToggle}
			/>
		</fieldset>
	);
}
