import { Input } from '@minimalstuff/ui';
import type { ChangeEvent } from 'react';

import { useDaemonStore } from '#/state/daemon_store';
import { summarizeTags } from '#/sidebar/summarize_tags';
import { SettingsSwitch } from '#/settings/settings_switch';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import type {
	SettingsTextField,
	SettingsToggleField,
} from '#/settings/settings_field_specs';

type SettingsStartupFieldsetProps = {
	values: SettingsFormValues;
	onChangeText: (field: SettingsTextField, value: string) => void;
	onChangeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
};

export function SettingsStartupFieldset({
	values,
	onChangeText,
	onChangeToggle,
}: Readonly<SettingsStartupFieldsetProps>) {
	const tunnels = useDaemonStore((state) => state.tunnels);
	const existingTags = summarizeTags(tunnels).map(({ tag }) => tag);
	const tagsHint =
		existingTags.length === 0
			? 'Séparés par des virgules.'
			: `Séparés par des virgules. Tags existants : ${existingTags.join(', ')}.`;

	const handleTagsChange = (event: ChangeEvent<HTMLInputElement>) =>
		onChangeText('startOnLaunchTags', event.target.value);

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">Démarrage</legend>
			<SettingsSwitch
				field="launchAtLogin"
				label="Lancer Tunneler à l'ouverture de session"
				hint="Démarre le daemon et son icône, sans ouvrir la fenêtre."
				isChecked={values.launchAtLogin}
				onChange={onChangeToggle}
			/>
			<SettingsSwitch
				field="restoreOnLaunch"
				label="Rétablir les tunnels actifs au lancement"
				hint="Relance les tunnels qui tournaient à la dernière fermeture."
				isChecked={values.restoreOnLaunch}
				onChange={onChangeToggle}
			/>
			<Input
				name="startOnLaunchTags"
				label="Tags démarrés au lancement"
				description={tagsHint}
				value={values.startOnLaunchTags}
				onChange={handleTagsChange}
				autoComplete="off"
			/>
		</fieldset>
	);
}
