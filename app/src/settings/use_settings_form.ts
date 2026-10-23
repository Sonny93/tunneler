import { useLingui } from '@lingui/react/macro';
import { useState, type SubmitEvent } from 'react';

import type { SettingsDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useOpenersDraft } from '#/openers/use_openers_draft';
import {
	countErrorsByTab,
	type SettingsTabErrorCounts,
} from '#/settings/count_errors_by_tab';
import { getSettingsFieldTab } from '#/settings/settings_field_specs';
import { toSettings } from '#/settings/settings_form_values';
import type { SettingsTab } from '#/settings/settings_tab';
import { useSettingsTabNavigation } from '#/settings/use_settings_tab_navigation';
import {
	useSettingsValues,
	type UseSettingsValuesReturn,
} from '#/settings/use_settings_values';
import {
	getFirstInvalidSettingsField,
	validateSettingsForm,
	type SettingsFormErrors,
} from '#/settings/validate_settings_form';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';

const OPENER_FORM_FIRST_FIELD = 'opener-name';

export type UseSettingsFormReturn = UseSettingsValuesReturn & {
	errors: SettingsFormErrors;
	errorCounts: SettingsTabErrorCounts;
	isSaving: boolean;
	/** Whether leaving now would lose a change: a setting, an application, or an application form with input. */
	isDirty: boolean;
	summary: ReturnType<typeof useFormErrorSummary>;
	openers: ReturnType<typeof useOpenersDraft>;
	activeTab: SettingsTab;
	selectTab: (tab: SettingsTab) => void;
	attachForm: (element: HTMLFormElement | null) => void;
	submit: (event: SubmitEvent<HTMLFormElement>) => void;
};

export function useSettingsForm(
	settings: Readonly<SettingsDto>,
	initialTab?: SettingsTab
): UseSettingsFormReturn {
	const { t } = useLingui();
	const gateway = useDaemonGateway();
	const settingsValues = useSettingsValues(settings);
	const openers = useOpenersDraft();
	const [errors, setErrors] = useState<SettingsFormErrors>({});
	const [isSaving, setIsSaving] = useState(false);
	const summary = useFormErrorSummary();
	const navigation = useSettingsTabNavigation(initialTab);

	const save = async () => {
		setIsSaving(true);
		summary.clearFailure();
		const nextSettings = toSettings(settingsValues.values, settings);
		try {
			const availableOpeners = await gateway.updateSettings(
				nextSettings,
				openers.userOpeners
			);
			useDaemonStore.getState().setSettings(nextSettings);
			useDaemonStore.getState().setOpeners(availableOpeners);
		} catch (rejection) {
			setIsSaving(false);
			summary.reportFailure(rejection);
			return;
		}
		useToastStore
			.getState()
			.pushToast({ tone: 'success', message: t`Settings saved` });
		useUiStore.getState().closePanel();
	};

	const submit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isSaving) return;
		summary.clearFailure();
		const foundErrors = validateSettingsForm(settingsValues.values);
		setErrors(foundErrors);
		const firstInvalidField = getFirstInvalidSettingsField(foundErrors);
		if (firstInvalidField !== null) {
			navigation.showField(
				getSettingsFieldTab(firstInvalidField),
				firstInvalidField
			);
			return;
		}
		if (openers.hasFormInput) {
			summary.reportMessagesKeepingFocus([
				t`An application form still has input. Add the application or cancel the form before saving.`,
			]);
			navigation.showField('opening', OPENER_FORM_FIRST_FIELD);
			return;
		}
		void save();
	};

	return {
		...settingsValues,
		errors,
		errorCounts: countErrorsByTab(errors),
		isSaving,
		isDirty:
			settingsValues.isChanged || openers.isChanged || openers.hasFormInput,
		summary,
		openers,
		activeTab: navigation.activeTab,
		selectTab: navigation.selectTab,
		attachForm: navigation.attachForm,
		submit,
	};
}
