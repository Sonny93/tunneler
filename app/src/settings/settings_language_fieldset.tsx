import { useLingui } from '@lingui/react/macro';
import { Select } from '@minimalstuff/ui';
import type { ChangeEvent } from 'react';

import type { LanguageSettingDto } from '#/daemon/protocol';
import { isSupportedLocale } from '#/i18n/supported_locales';

const SYSTEM_LANGUAGE = 'system' satisfies LanguageSettingDto;

type SettingsLanguageFieldsetProps = {
	value: LanguageSettingDto;
	onChange: (language: LanguageSettingDto) => void;
};

/** The display language; each language is named in itself so it can be found whatever the current one. */
export function SettingsLanguageFieldset({
	value,
	onChange,
}: Readonly<SettingsLanguageFieldsetProps>) {
	const { t } = useLingui();
	const options = [
		{ value: SYSTEM_LANGUAGE, label: t`System` },
		{ value: 'en', label: 'English' },
		{ value: 'fr', label: 'Français' },
	] as const;

	const handleChange = (event: ChangeEvent<HTMLSelectElement>) => {
		const selected = event.target.value;
		if (selected === SYSTEM_LANGUAGE || isSupportedLocale(selected)) {
			onChange(selected);
		}
	};

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">{t`Language`}</legend>
			<Select
				name="language"
				label={t`Display language`}
				options={options.map((option) => ({ ...option }))}
				value={value}
				onChange={handleChange}
			/>
		</fieldset>
	);
}
