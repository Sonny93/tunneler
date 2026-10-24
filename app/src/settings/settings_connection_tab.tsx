import { CONNECTION_SECTIONS } from '#/settings/settings_field_specs';
import { SettingsFieldset } from '#/settings/settings_fieldset';
import { SettingsTabContent } from '#/settings/settings_tab_content';
import { SettingsX11ServerStart } from '#/settings/settings_x11_server_start';
import type { UseSettingsFormReturn } from '#/settings/use_settings_form';

const X11_SECTION_NAME = 'x11';

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
			>
				{section.name === X11_SECTION_NAME && (
					<SettingsX11ServerStart
						value={form.values.x11ServerStart}
						onChange={form.changeX11ServerStart}
					/>
				)}
			</SettingsFieldset>
		))}
	</SettingsTabContent>
);
