import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';

import type { SettingsToggleField } from '#/settings/settings_field_specs';
import type { SettingsFormValues } from '#/settings/settings_form_values';
import { SettingsSwitch } from '#/settings/settings_switch';
import { useForgetSecrets } from '#/settings/use_forget_secrets';

type SettingsSecretsSectionProps = {
	values: SettingsFormValues;
	onChangeToggle: (field: SettingsToggleField, isEnabled: boolean) => void;
};

export function SettingsSecretsSection({
	values,
	onChangeToggle,
}: Readonly<SettingsSecretsSectionProps>) {
	const { t } = useLingui();
	const forgetSecrets = useForgetSecrets();

	return (
		<div className="flex flex-col gap-4">
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
		</div>
	);
}
