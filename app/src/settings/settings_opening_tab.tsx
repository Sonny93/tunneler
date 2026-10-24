import { ApplicationsSection } from '#/openers/applications_section';
import { SettingsOpenFieldset } from '#/settings/settings_open_fieldset';
import { SettingsTabContent } from '#/settings/settings_tab_content';
import type { UseSettingsFormReturn } from '#/settings/use_settings_form';

type SettingsOpeningTabProps = {
	form: UseSettingsFormReturn;
};

export const SettingsOpeningTab = ({
	form,
}: Readonly<SettingsOpeningTabProps>) => (
	<SettingsTabContent>
		<SettingsOpenFieldset
			host={form.values.openHost}
			onHostChange={form.changeOpenHost}
			openDefault={form.values.openDefault}
			onOpenDefaultChange={form.changeOpenDefault}
		/>
		<ApplicationsSection draft={form.openers} />
	</SettingsTabContent>
);
