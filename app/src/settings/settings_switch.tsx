import { Switch } from '@minimalstuff/ui';
import type { ChangeEvent } from 'react';

import type { SettingsToggleField } from '#/settings/settings_field_specs';

type SettingsSwitchProps = {
	field: SettingsToggleField;
	label: string;
	hint: string;
	isChecked: boolean;
	onChange: (field: SettingsToggleField, isEnabled: boolean) => void;
};

export function SettingsSwitch({
	field,
	label,
	hint,
	isChecked,
	onChange,
}: Readonly<SettingsSwitchProps>) {
	const handleChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChange(field, event.target.checked);

	return (
		<Switch
			name={field}
			label={label}
			description={hint}
			checked={isChecked}
			onChange={handleChange}
		/>
	);
}
