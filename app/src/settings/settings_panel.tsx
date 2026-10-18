import { useId } from 'react';
import { Button } from '@minimalstuff/ui';

import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import type { SettingsDto } from '#/daemon/protocol';
import { SettingsSwitch } from '#/settings/settings_switch';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useSettingsForm } from '#/settings/use_settings_form';
import { SettingsFieldset } from '#/settings/settings_fieldset';
import { SETTINGS_SECTIONS } from '#/settings/settings_field_specs';
import { OpenConfigFolderButton } from '#/settings/open_config_folder_button';
import { SettingsSecretsFieldset } from '#/settings/settings_secrets_fieldset';
import { SettingsStartupFieldset } from '#/settings/settings_startup_fieldset';

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
				<fieldset className="border-0 p-0">
					<legend className="mb-3 text-sm font-semibold">Santé</legend>
					<SettingsSwitch
						field="healthProbe"
						label="Sonder la cible périodiquement"
						hint="Uniquement quand cette fenêtre est ouverte"
						isChecked={values.healthProbe}
						onChange={form.changeToggle}
					/>
				</fieldset>
				<SettingsStartupFieldset
					values={values}
					onChangeText={form.changeText}
					onChangeToggle={form.changeToggle}
				/>
				<SettingsSecretsFieldset
					values={values}
					onChangeToggle={form.changeToggle}
				/>
			</form>
			<OpenConfigFolderButton />
		</Drawer>
	);
}
