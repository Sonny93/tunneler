import { useLingui } from '@lingui/react/macro';
import { useState, type SubmitEvent } from 'react';

import type {
	OpenHostDto,
	SettingsDto,
	StandbyNotifyDto,
	StandbySoundDto,
	LanguageSettingDto,
} from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import type {
	SettingsTextField,
	SettingsToggleField,
} from '#/settings/settings_field_specs';
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
import { useToastStore } from '#/shared/toasts/toast_store';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';

type UseSettingsFormReturn = {
	values: SettingsFormValues;
	errors: SettingsFormErrors;
	isSaving: boolean;
	summary: ReturnType<typeof useFormErrorSummary>;
	changeText: (field: SettingsTextField, value: string) => void;
	changeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
	changeOpenHost: (host: OpenHostDto) => void;
	changeStandbyNotify: (mode: StandbyNotifyDto) => void;
	changeStandbySound: (sound: StandbySoundDto) => void;
	changeLanguage: (language: LanguageSettingDto) => void;
	submit: (event: SubmitEvent<HTMLFormElement>) => void;
};

function focusField(form: HTMLFormElement, fieldName: string): void {
	const field = form.elements.namedItem(fieldName);
	if (field instanceof HTMLElement) field.focus();
}

export function useSettingsForm(
	settings: Readonly<SettingsDto>
): UseSettingsFormReturn {
	const { t } = useLingui();
	const gateway = useDaemonGateway();
	const [values, setValues] = useState(() => toSettingsFormValues(settings));
	const [errors, setErrors] = useState<SettingsFormErrors>({});
	const [isSaving, setIsSaving] = useState(false);
	const summary = useFormErrorSummary();

	const changeText = (field: SettingsTextField, value: string) =>
		setValues((current) => ({ ...current, [field]: value }));
	const changeToggle = (field: SettingsToggleField, isEnabled: boolean) =>
		setValues((current) => ({ ...current, [field]: isEnabled }));

	const changeOpenHost = (openHost: OpenHostDto) =>
		setValues((current) => ({ ...current, openHost }));

	const changeStandbyNotify = (standbyNotify: StandbyNotifyDto) =>
		setValues((current) => ({ ...current, standbyNotify }));

	const changeStandbySound = (standbySound: StandbySoundDto) =>
		setValues((current) => ({ ...current, standbySound }));

	const changeLanguage = (language: LanguageSettingDto) =>
		setValues((current) => ({ ...current, language }));

	const save = async () => {
		setIsSaving(true);
		summary.clearFailure();
		const nextSettings = toSettings(values, settings);
		try {
			await gateway.updateSettings(nextSettings);
		} catch (rejection) {
			setIsSaving(false);
			summary.reportFailure(rejection);
			return;
		}
		useDaemonStore.getState().setSettings(nextSettings);
		useToastStore
			.getState()
			.pushToast({ tone: 'success', message: t`Settings saved` });
		useUiStore.getState().closePanel();
	};

	const submit = (event: SubmitEvent<HTMLFormElement>) => {
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
		changeToggle,
		changeOpenHost,
		changeStandbyNotify,
		changeStandbySound,
		changeLanguage,
		submit,
	};
}
