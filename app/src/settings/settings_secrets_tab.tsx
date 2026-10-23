import { SettingsSecretsSection } from '#/settings/settings_secrets_section';
import { SettingsTabContent } from '#/settings/settings_tab_content';
import type { UseSettingsFormReturn } from '#/settings/use_settings_form';

type SettingsSecretsTabProps = {
	form: UseSettingsFormReturn;
};

export const SettingsSecretsTab = ({
	form,
}: Readonly<SettingsSecretsTabProps>) => (
	<SettingsTabContent>
		<SettingsSecretsSection
			values={form.values}
			onChangeToggle={form.changeToggle}
		/>
	</SettingsTabContent>
);
