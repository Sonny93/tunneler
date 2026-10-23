import { useEffect, useState } from 'react';

import {
	DEFAULT_SETTINGS_TAB,
	type SettingsTab,
} from '#/settings/settings_tab';

type FieldFocusRequest = { fieldName: string };

type UseSettingsTabNavigationReturn = {
	activeTab: SettingsTab;
	/** Callback ref for the `<form>` that holds every field. */
	attachForm: (element: HTMLFormElement | null) => void;
	selectTab: (tab: SettingsTab) => void;
	/** Shows `tab` and focuses the field once the panel is visible again. */
	showField: (tab: SettingsTab, fieldName: string) => void;
};

/** The active tab of the settings, and focus moves that must wait for a hidden panel to show. */
export function useSettingsTabNavigation(
	initialTab: SettingsTab = DEFAULT_SETTINGS_TAB
): UseSettingsTabNavigationReturn {
	const [activeTab, setActiveTab] = useState(initialTab);
	const [focusRequest, setFocusRequest] = useState<FieldFocusRequest | null>(
		null
	);
	const [formElement, attachForm] = useState<HTMLFormElement | null>(null);

	useEffect(() => {
		if (focusRequest === null || formElement === null) return;
		const field = formElement.elements.namedItem(focusRequest.fieldName);
		if (field instanceof HTMLElement) field.focus();
	}, [focusRequest, formElement]);

	const showField = (tab: SettingsTab, fieldName: string) => {
		setActiveTab(tab);
		setFocusRequest({ fieldName });
	};

	return { activeTab, attachForm, selectTab: setActiveTab, showField };
}
