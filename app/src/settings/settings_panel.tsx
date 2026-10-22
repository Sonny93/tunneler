import { useId } from 'react';
import { Button } from '@minimalstuff/ui';

import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import type { SettingsDto } from '#/daemon/protocol';
import { SettingsSwitch } from '#/settings/settings_switch';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useSettingsForm } from '#/settings/use_settings_form';
import { SettingsFieldset } from '#/settings/settings_fieldset';
import { ApplicationsSection } from '#/openers/applications_section';
import { StandbySoundOptions } from '#/settings/standby_sound_options';
import { StandbyNotifyOptions } from '#/settings/standby_notify_options';
import { SettingsOpenFieldset } from '#/settings/settings_open_fieldset';
import { OpenConfigFolderButton } from '#/settings/open_config_folder_button';
import { SettingsSecretsFieldset } from '#/settings/settings_secrets_fieldset';
import { SettingsStartupFieldset } from '#/settings/settings_startup_fieldset';
import { SettingsUpdatesFieldset } from '#/settings/settings_updates_fieldset';
import {
	SETTINGS_SECTIONS,
	STANDBY_SECTION,
} from '#/settings/settings_field_specs';

type SettingsPanelProps = {
	settings: SettingsDto;
};

export function SettingsPanel({ settings }: Readonly<SettingsPanelProps>) {
	const formId = useId();
	const closePanel = useUiStore((state) => state.closePanel);
	const { values, errors, isSaving, summary, ...form } =
		useSettingsForm(settings);

	const footer = (
		<div className="flex justify-end gap-2">
			<Button variant="outline" color="neutral" onClick={closePanel}>
				Annuler
			</Button>
			<Button type="submit" form={formId} loading={isSaving}>
				Enregistrer
			</Button>
		</div>
	);

	return (
		<Drawer isOpen title="Paramètres" onClose={closePanel} footer={footer}>
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
				{SETTINGS_SECTIONS.map((section) => (
					<SettingsFieldset
						key={section.legend}
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
						label="Mettre les tunnels en veille"
						hint="Les tunnels locaux et dynamiques gardent leur port sans session SSH jusqu'à la première connexion. Prend effet au prochain démarrage de chaque tunnel."
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
