import { OpenConfigFolderButton } from '#/settings/open_config_folder_button';
import { SettingsLanguageFieldset } from '#/settings/settings_language_fieldset';
import { SettingsStartupFieldset } from '#/settings/settings_startup_fieldset';
import { SettingsTabContent } from '#/settings/settings_tab_content';
import { SettingsUpdatesFieldset } from '#/settings/settings_updates_fieldset';
import type { UseSettingsFormReturn } from '#/settings/use_settings_form';

type SettingsGeneralTabProps = {
	form: UseSettingsFormReturn;
};

export const SettingsGeneralTab = ({
	form,
}: Readonly<SettingsGeneralTabProps>) => (
	<SettingsTabContent>
		<SettingsLanguageFieldset
			value={form.values.language}
			onChange={form.changeLanguage}
		/>
		<SettingsStartupFieldset
			values={form.values}
			onChangeText={form.changeText}
			onChangeToggle={form.changeToggle}
		/>
		<SettingsUpdatesFieldset
			values={form.values}
			onChangeToggle={form.changeToggle}
		/>
		<OpenConfigFolderButton />
	</SettingsTabContent>
);
