import { useState, type FormEvent } from 'react';

import { useUiStore } from '#/state/ui_store';
import type { SettingsDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import type { SettingsTextField } from '#/settings/settings_field_specs';
import {
	toSettings,
	toSettingsFormValues,
	type SettingsFormValues,
} from '#/settings/settings_form_values';
import {
	getFirstInvalidSettingsField,
	validateSettingsForm,
	type SettingsFormErrors,
} from '#/settings/validate_settings_form';

type UseSettingsFormReturn = {
	values: SettingsFormValues;
	errors: SettingsFormErrors;
	isSaving: boolean;
	summary: ReturnType<typeof useFormErrorSummary>;
	changeText: (field: SettingsTextField, value: string) => void;
	changeHealthProbe: (isEnabled: boolean) => void;
	submit: (event: FormEvent<HTMLFormElement>) => void;
};

function focusField(form: HTMLFormElement, fieldName: string): void {
	const field = form.elements.namedItem(fieldName);
	if (field instanceof HTMLElement) field.focus();
}

export function useSettingsForm(
	settings: Readonly<SettingsDto>
): UseSettingsFormReturn {
	const gateway = useDaemonGateway();
	const [values, setValues] = useState(() => toSettingsFormValues(settings));
	const [errors, setErrors] = useState<SettingsFormErrors>({});
	const [isSaving, setIsSaving] = useState(false);
	const summary = useFormErrorSummary();

	const changeText = (field: SettingsTextField, value: string) =>
		setValues((current) => ({ ...current, [field]: value }));
	const changeHealthProbe = (healthProbe: boolean) =>
		setValues((current) => ({ ...current, healthProbe }));

	const save = async () => {
		setIsSaving(true);
		summary.clearFailure();
		try {
			await gateway.updateSettings(toSettings(values, settings));
		} catch (rejection) {
			setIsSaving(false);
			summary.reportFailure(rejection);
			return;
		}
		useToastStore
			.getState()
			.pushToast({ tone: 'success', message: 'Paramètres enregistrés' });
		useUiStore.getState().closePanel();
	};

	const submit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isSaving) return;
		const foundErrors = validateSettingsForm(values);
		setErrors(foundErrors);
		const firstInvalidField = getFirstInvalidSettingsField(foundErrors);
		if (firstInvalidField !== null) {
			focusField(event.currentTarget, firstInvalidField);
			return;
		}
		void save();
	};

	return {
		values,
		errors,
		isSaving,
		summary,
		changeText,
		changeHealthProbe,
		submit,
	};
}
