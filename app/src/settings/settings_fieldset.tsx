import { Input } from '@minimalstuff/ui';
import { useId, type ChangeEvent } from 'react';

import { FormHint } from '#/shared/form_hint';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import type { SettingsFormErrors } from '#/settings/validate_settings_form';
import type {
	SettingsFieldSpec,
	SettingsSectionSpec,
	SettingsTextField,
} from '#/settings/settings_field_specs';

type SettingsFieldProps = {
	spec: SettingsFieldSpec;
	value: string;
	error?: string;
	onChange: (field: SettingsTextField, value: string) => void;
};

function SettingsField({
	spec,
	value,
	error,
	onChange,
}: Readonly<SettingsFieldProps>) {
	const hintId = useId();

	const handleChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChange(spec.field, event.target.value);

	return (
		<div>
			<Input
				name={spec.field}
				label={spec.label}
				value={value}
				onChange={handleChange}
				error={error}
				required={!spec.isOptional}
				inputMode={spec.inputMode}
				autoComplete="off"
				aria-describedby={spec.hint ? hintId : undefined}
			/>
			{spec.hint && <FormHint id={hintId}>{spec.hint}</FormHint>}
		</div>
	);
}

type SettingsFieldsetProps = {
	section: SettingsSectionSpec;
	values: SettingsFormValues;
	errors: SettingsFormErrors;
	onChange: (field: SettingsTextField, value: string) => void;
};

export const SettingsFieldset = ({
	section,
	values,
	errors,
	onChange,
}: Readonly<SettingsFieldsetProps>) => (
	<fieldset className="flex flex-col gap-4 border-0 p-0">
		<legend className="mb-3 text-sm font-semibold">{section.legend}</legend>
		{section.fields.map((spec) => (
			<SettingsField
				key={spec.field}
				spec={spec}
				value={values[spec.field]}
				error={errors[spec.field]}
				onChange={onChange}
			/>
		))}
	</fieldset>
);
