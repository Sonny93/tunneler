import { useLingui } from '@lingui/react/macro';
import { Tabs, type TabItem } from '@minimalstuff/ui';

import { SettingsConnectionTab } from '#/settings/settings_connection_tab';
import { SettingsGeneralTab } from '#/settings/settings_general_tab';
import { SettingsOpeningTab } from '#/settings/settings_opening_tab';
import { SettingsSecretsTab } from '#/settings/settings_secrets_tab';
import { SettingsStandbyTab } from '#/settings/settings_standby_tab';
import { SETTINGS_TABS } from '#/settings/settings_tab';
import { SettingsTabErrorBadge } from '#/settings/settings_tab_error_badge';
import type { UseSettingsFormReturn } from '#/settings/use_settings_form';

type SettingsTabsProps = {
	form: UseSettingsFormReturn;
};

/** Every panel stays mounted so a half-filled field survives a tab switch and stays in the form. */
export function SettingsTabs({ form }: Readonly<SettingsTabsProps>) {
	const { t } = useLingui();

	const buildItem = (
		title: string,
		tab: (typeof SETTINGS_TABS)[number],
		content: TabItem['content']
	): TabItem => {
		const errorCount = form.errorCounts[tab] ?? 0;
		return {
			title,
			content,
			badge:
				errorCount > 0 ? (
					<SettingsTabErrorBadge count={errorCount} />
				) : undefined,
		};
	};

	const items = [
		buildItem(t`General`, 'general', <SettingsGeneralTab form={form} />),
		buildItem(
			t`Connection`,
			'connection',
			<SettingsConnectionTab form={form} />
		),
		buildItem(t`Standby`, 'standby', <SettingsStandbyTab form={form} />),
		buildItem(t`Opening`, 'opening', <SettingsOpeningTab form={form} />),
		buildItem(t`Secrets`, 'secrets', <SettingsSecretsTab form={form} />),
	];

	const handleChange = (index: number) => {
		const tab = SETTINGS_TABS[index];
		if (tab !== undefined) form.selectTab(tab);
	};

	return (
		<Tabs
			items={items}
			orientation="vertical"
			keepMounted
			value={SETTINGS_TABS.indexOf(form.activeTab)}
			onChange={handleChange}
			className="min-h-0 flex-1"
			panelClassName="pr-1"
		/>
	);
}
