import { CONNECTION_SECTIONS } from '#/settings/settings_field_specs';
import { SettingsFieldset } from '#/settings/settings_fieldset';
import { SettingsTabContent } from '#/settings/settings_tab_content';
import type { UseSettingsFormReturn } from '#/settings/use_settings_form';

type SettingsConnectionTabProps = {
	form: UseSettingsFormReturn;
};

export const SettingsConnectionTab = ({
	form,
}: Readonly<SettingsConnectionTabProps>) => (
	<SettingsTabContent>
		{CONNECTION_SECTIONS.map((section) => (
			<SettingsFieldset
				key={section.name}
				section={section}
				values={form.values}
				errors={form.errors}
				onChange={form.changeText}
			/>
		))}
	</SettingsTabContent>
);
