import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';

import type { SettingsToggleField } from '#/settings/settings_field_specs';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import { SettingsSwitch } from '#/settings/settings_switch';
import { useForgetSecrets } from '#/settings/use_forget_secrets';

type SettingsSecretsFieldsetProps = {
	values: SettingsFormValues;
	onChangeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
};

export function SettingsSecretsFieldset({
	values,
	onChangeToggle,
}: Readonly<SettingsSecretsFieldsetProps>) {
	const { t } = useLingui();
	const forgetSecrets = useForgetSecrets();

	return (
		<fieldset className="flex flex-col gap-4 border-0 p-0">
			<legend className="mb-3 text-sm font-semibold">{t`Secrets`}</legend>
			<SettingsSwitch
				field="useOsKeyring"
				label={t`Remember passphrases in the system keyring`}
				hint={t`Windows: Credential Manager; Linux: Secret Service (GNOME Keyring, KWallet). When off, passphrases are remembered only until Tunneler is closed.`}
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
					{t`Forget remembered passphrases`}
				</Button>
			</div>
		</fieldset>
	);
}
