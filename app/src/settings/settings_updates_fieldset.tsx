import { SettingsSwitch } from '#/settings/settings_switch';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import type { SettingsToggleField } from '#/settings/settings_field_specs';

type SettingsUpdatesFieldsetProps = {
	values: SettingsFormValues;
	onChangeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
};

export function SettingsUpdatesFieldset({
	values,
	onChangeToggle,
}: Readonly<SettingsUpdatesFieldsetProps>) {
	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">Mises à jour</legend>
			<SettingsSwitch
				field="checkForUpdates"
				label="Vérifier les mises à jour à l'ouverture"
				hint="À chaque ouverture de la fenêtre, Tunneler contacte GitHub pour savoir si une nouvelle version existe. Rien n'est installé sans votre accord."
				isChecked={values.checkForUpdates}
				onChange={onChangeToggle}
			/>
		</fieldset>
	);
}
