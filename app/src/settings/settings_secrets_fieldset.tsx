import { Button } from '@minimalstuff/ui';

import { SettingsSwitch } from '#/settings/settings_switch';
import { useForgetSecrets } from '#/settings/use_forget_secrets';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import type { SettingsToggleField } from '#/settings/settings_field_specs';

type SettingsSecretsFieldsetProps = {
	values: SettingsFormValues;
	onChangeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
};

export function SettingsSecretsFieldset({
	values,
	onChangeToggle,
}: Readonly<SettingsSecretsFieldsetProps>) {
	const forgetSecrets = useForgetSecrets();

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">Secrets</legend>
			<SettingsSwitch
				field="useOsKeyring"
				label="Mémoriser les passphrases dans le trousseau du système"
				hint="Windows : gestionnaire d'identification ; Linux : Secret Service (GNOME Keyring, KWallet). Désactivé : mémorisation jusqu'à la fermeture de Tunneler seulement."
				isChecked={values.useOsKeyring}
				onChange={onChangeToggle}
			/>
			<div>
				<Button
					type="button"
					variant="outline"
					color="neutral"
					onClick={() => void forgetSecrets()}
				>
					Oublier les passphrases mémorisées
				</Button>
			</div>
		</fieldset>
	);
}
