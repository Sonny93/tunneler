import { useLingui } from '@lingui/react/macro';
import { ConfirmModal } from '@minimalstuff/ui';

import { useDaemonAction } from '#/daemon/use_daemon_action';
import { getForgottenSecretsMessage } from '#/settings/forgotten_secrets_message';
import { useToastStore } from '#/shared/toasts/toast_store';

/** Asks for confirmation, then erases every remembered passphrase and toasts how many. */
export function useForgetSecrets(): () => Promise<void> {
	const { t, i18n } = useLingui();
	const runDaemonAction = useDaemonAction();

	return async () => {
		const isConfirmed = await ConfirmModal.call({
			title: t`Forget remembered passphrases?`,
			children: t`Passphrases kept in memory and in the system keyring will be erased; they will be asked for again at the next connection.`,
			confirmLabel: t`Forget`,
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		await runDaemonAction(
			async (gateway) => {
				const { forgottenCount } = await gateway.forgetSecrets();
				useToastStore.getState().pushToast({
					tone: 'success',
					message: getForgottenSecretsMessage(forgottenCount, i18n),
				});
			},
			t`Couldn't forget the passphrases`
		);
	};
}
