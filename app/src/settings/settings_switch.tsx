import { Switch } from '@minimalstuff/ui';
import { useId, type ChangeEvent } from 'react';

import { FormHint } from '#/shared/form_hint';
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
	const hintId = useId();

	const handleChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChange(field, event.target.checked);

	return (
		<div>
			<Switch
				name={field}
				label={label}
				checked={isChecked}
				onChange={handleChange}
				aria-describedby={hintId}
			/>
			<FormHint id={hintId}>{hint}</FormHint>
		</div>
	);
}
