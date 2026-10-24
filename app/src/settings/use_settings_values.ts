import { useState } from 'react';

import type {
	LanguageSettingDto,
	OpenDefaultDto,
	OpenHostDto,
	SettingsDto,
	StandbyNotifyDto,
	StandbySoundDto,
	X11ServerStartDto,
} from '#/daemon/protocol';
import type {
	SettingsTextField,
	SettingsToggleField,
} from '#/settings/settings_field_specs';
import {
	toSettingsFormValues,
	type SettingsFormValues,
} from '#/settings/settings_form_values';
import { areRecordsEqual } from '#/shared/are_records_equal';

export type UseSettingsValuesReturn = {
	values: SettingsFormValues;
	isChanged: boolean;
	changeText: (field: SettingsTextField, value: string) => void;
	changeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
	changeOpenHost: (host: OpenHostDto) => void;
	changeOpenDefault: (openDefault: OpenDefaultDto) => void;
	changeX11ServerStart: (x11ServerStart: X11ServerStartDto) => void;
	changeStandbyNotify: (mode: StandbyNotifyDto) => void;
	changeStandbySound: (sound: StandbySoundDto) => void;
	changeStandbyVolume: (volume: number) => void;
	changeLanguage: (language: LanguageSettingDto) => void;
};

/** The values of the settings fields, starting from the stored settings. */
export function useSettingsValues(
	settings: Readonly<SettingsDto>
): UseSettingsValuesReturn {
	const [initialValues] = useState(() => toSettingsFormValues(settings));
	const [values, setValues] = useState(initialValues);

	const changeText = (field: SettingsTextField, value: string) =>
		setValues((current) => ({ ...current, [field]: value }));
	const changeToggle = (field: SettingsToggleField, isEnabled: boolean) =>
		setValues((current) => ({ ...current, [field]: isEnabled }));
	const changeOpenHost = (openHost: OpenHostDto) =>
		setValues((current) => ({ ...current, openHost }));
	const changeOpenDefault = (openDefault: OpenDefaultDto) =>
		setValues((current) => ({ ...current, openDefault }));
	const changeX11ServerStart = (x11ServerStart: X11ServerStartDto) =>
		setValues((current) => ({ ...current, x11ServerStart }));
	const changeStandbyNotify = (standbyNotify: StandbyNotifyDto) =>
		setValues((current) => ({ ...current, standbyNotify }));
	const changeStandbySound = (standbySound: StandbySoundDto) =>
		setValues((current) => ({ ...current, standbySound }));
	const changeStandbyVolume = (standbyVolume: number) =>
		setValues((current) => ({ ...current, standbyVolume }));
	const changeLanguage = (language: LanguageSettingDto) =>
		setValues((current) => ({ ...current, language }));

	return {
		values,
		isChanged: !areRecordsEqual(values, initialValues),
		changeText,
		changeToggle,
		changeOpenHost,
		changeOpenDefault,
		changeX11ServerStart,
		changeStandbyNotify,
		changeStandbySound,
		changeStandbyVolume,
		changeLanguage,
	};
}
