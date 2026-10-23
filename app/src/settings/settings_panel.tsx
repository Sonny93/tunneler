import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';
import { useId } from 'react';

import type { SettingsDto } from '#/daemon/protocol';
import { ApplicationsSection } from '#/openers/applications_section';
import { OpenConfigFolderButton } from '#/settings/open_config_folder_button';
import {
	SETTINGS_SECTIONS,
	STANDBY_SECTION,
} from '#/settings/settings_field_specs';
import { SettingsFieldset } from '#/settings/settings_fieldset';
import { SettingsLanguageFieldset } from '#/settings/settings_language_fieldset';
import { SettingsOpenFieldset } from '#/settings/settings_open_fieldset';
import { SettingsSecretsFieldset } from '#/settings/settings_secrets_fieldset';
import { SettingsStartupFieldset } from '#/settings/settings_startup_fieldset';
import { SettingsSwitch } from '#/settings/settings_switch';
import { SettingsUpdatesFieldset } from '#/settings/settings_updates_fieldset';
import { StandbyNotifyOptions } from '#/settings/standby_notify_options';
import { StandbySoundOptions } from '#/settings/standby_sound_options';
import { useSettingsForm } from '#/settings/use_settings_form';
import { Drawer } from '#/shared/drawer';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useUiStore } from '#/state/ui_store';

type SettingsPanelProps = {
	settings: SettingsDto;
};

export function SettingsPanel({ settings }: Readonly<SettingsPanelProps>) {
	const { t } = useLingui();
	const formId = useId();
	const closePanel = useUiStore((state) => state.closePanel);
	const { values, errors, isSaving, summary, ...form } =
		useSettingsForm(settings);

	const footer = (
		<div className="flex justify-end gap-2">
			<Button variant="outline" color="neutral" onClick={closePanel}>
				{t`Cancel`}
			</Button>
			<Button type="submit" form={formId} loading={isSaving}>
				{t`Save`}
			</Button>
		</div>
	);

	return (
		<Drawer isOpen title={t`Settings`} onClose={closePanel} footer={footer}>
			<form
				id={formId}
				noValidate
				onSubmit={form.submit}
				className="flex flex-col gap-6"
			>
				<FormErrorSummary
					messages={summary.messages}
					attach={summary.attachSummary}
				/>
				<SettingsLanguageFieldset
					value={values.language}
					onChange={form.changeLanguage}
				/>
				{SETTINGS_SECTIONS.map((section) => (
					<SettingsFieldset
						key={section.name}
						section={section}
						values={values}
						errors={errors}
						onChange={form.changeText}
					/>
				))}
				<SettingsFieldset
					section={STANDBY_SECTION}
					values={values}
					errors={errors}
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
				<SettingsOpenFieldset
					value={values.openHost}
					onChange={form.changeOpenHost}
				/>
				<SettingsStartupFieldset
					values={values}
					onChangeText={form.changeText}
					onChangeToggle={form.changeToggle}
				/>
				<SettingsSecretsFieldset
					values={values}
					onChangeToggle={form.changeToggle}
				/>
				<SettingsUpdatesFieldset
					values={values}
					onChangeToggle={form.changeToggle}
				/>
			</form>
			<ApplicationsSection />
			<OpenConfigFolderButton />
		</Drawer>
	);
}
