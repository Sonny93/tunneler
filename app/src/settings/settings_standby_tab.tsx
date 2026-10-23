import { useLingui } from '@lingui/react/macro';

import { STANDBY_SECTION } from '#/settings/settings_field_specs';
import { SettingsFieldset } from '#/settings/settings_fieldset';
import { SettingsSwitch } from '#/settings/settings_switch';
import { SettingsTabContent } from '#/settings/settings_tab_content';
import { StandbyNotifyOptions } from '#/settings/standby_notify_options';
import { StandbySoundOptions } from '#/settings/standby_sound_options';
import type { UseSettingsFormReturn } from '#/settings/use_settings_form';

type SettingsStandbyTabProps = {
	form: UseSettingsFormReturn;
};

export function SettingsStandbyTab({
	form,
}: Readonly<SettingsStandbyTabProps>) {
	const { t } = useLingui();
	const { values } = form;

	return (
		<SettingsTabContent>
			<SettingsFieldset
				section={STANDBY_SECTION}
				values={values}
				errors={form.errors}
				onChange={form.changeText}
			>
				<SettingsSwitch
					field="standby"
					label={t`Put tunnels on standby`}
					hint={t`Local and dynamic tunnels keep their port without an SSH session until the first connection. Takes effect the next time each tunnel starts.`}
					isChecked={values.standby}
					onChange={form.changeToggle}
				/>
				<StandbyNotifyOptions
					value={values.standbyNotify}
					onChange={form.changeStandbyNotify}
				/>
				<StandbySoundOptions
					value={values.standbySound}
					notifyMode={values.standbyNotify}
					onChange={form.changeStandbySound}
				/>
			</SettingsFieldset>
		</SettingsTabContent>
	);
}
