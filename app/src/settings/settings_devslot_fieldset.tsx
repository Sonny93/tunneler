import { useLingui } from '@lingui/react/macro';
import { RadioOptions } from '@minimalstuff/ui';

import type { DevslotStalePolicyDto } from '#/daemon/protocol';
import type { SettingsToggleField } from '#/settings/settings_field_specs';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import { SettingsSwitch } from '#/settings/settings_switch';

type SettingsDevslotFieldsetProps = {
	values: SettingsFormValues;
	onChangeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
	onChangeStale: (policy: DevslotStalePolicyDto) => void;
};

export function SettingsDevslotFieldset({
	values,
	onChangeToggle,
	onChangeStale,
}: Readonly<SettingsDevslotFieldsetProps>) {
	const { t } = useLingui();
	const staleOptions: { value: DevslotStalePolicyDto; label: string }[] = [
		{ value: 'ask', label: t`Ask me` },
		{ value: 'delete', label: t`Delete them` },
	];

	const handleStaleChange = (selected: string) => {
		const option = staleOptions.find(
			(candidate) => candidate.value === selected
		);
		if (option) onChangeStale(option.value);
	};

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">{t`devslot`}</legend>
			<SettingsSwitch
				field="devslotPersist"
				label={t`Keep discovered tunnels by default`}
				hint={t`The "Keep these tunnels" switch of the devslot import starts on. A host can override it in the configuration file.`}
				isChecked={values.devslotPersist}
				onChange={onChangeToggle}
			/>
			<RadioOptions
				name="devslotStale"
				label={t`Saved tunnels whose port is no longer listed`}
				options={staleOptions}
				value={values.devslotStale}
				onChange={handleStaleChange}
			/>
		</fieldset>
	);
}
