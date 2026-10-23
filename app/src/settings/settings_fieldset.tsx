import type { MessageDescriptor } from '@lingui/core';
import { useLingui } from '@lingui/react/macro';
import { Input } from '@minimalstuff/ui';
import type { ChangeEvent, ReactNode } from 'react';

import type {
	SettingsFieldSpec,
	SettingsSectionSpec,
	SettingsTextField,
} from '#/settings/settings_field_specs';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import type { SettingsFormErrors } from '#/settings/validate_settings_form';

type SettingsFieldProps = {
	spec: SettingsFieldSpec;
	value: string;
	error?: MessageDescriptor;
	onChange: (field: SettingsTextField, value: string) => void;
};

function SettingsField({
	spec,
	value,
	error,
	onChange,
}: Readonly<SettingsFieldProps>) {
	const { i18n } = useLingui();
	const handleChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChange(spec.field, event.target.value);

	return (
		<Input
			name={spec.field}
			label={i18n._(spec.label)}
			value={value}
			onChange={handleChange}
			error={error && i18n._(error)}
			description={spec.hint && i18n._(spec.hint)}
			required={!spec.isOptional}
			inputMode={spec.inputMode}
			autoComplete="off"
		/>
	);
}

type SettingsFieldsetProps = {
	section: SettingsSectionSpec;
	values: SettingsFormValues;
	errors: SettingsFormErrors;
	onChange: (field: SettingsTextField, value: string) => void;
	/** Controls shown above the text fields, such as a switch. */
	children?: ReactNode;
};

export function SettingsFieldset({
	section,
	values,
	errors,
	onChange,
	children,
}: Readonly<SettingsFieldsetProps>) {
	const { i18n } = useLingui();

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">
				{i18n._(section.legend)}
			</legend>
			{children}
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
}
