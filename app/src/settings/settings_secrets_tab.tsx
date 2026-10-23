import { SettingsSecretsFieldset } from '#/settings/settings_secrets_fieldset';
import { SettingsTabContent } from '#/settings/settings_tab_content';
import type { UseSettingsFormReturn } from '#/settings/use_settings_form';

type SettingsSecretsTabProps = {
	form: UseSettingsFormReturn;
};

export const SettingsSecretsTab = ({
	form,
}: Readonly<SettingsSecretsTabProps>) => (
	<SettingsTabContent>
		<SettingsSecretsFieldset
			values={form.values}
			onChangeToggle={form.changeToggle}
		/>
	</SettingsTabContent>
);
