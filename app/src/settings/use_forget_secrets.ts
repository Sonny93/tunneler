import { ConfirmModal } from '@minimalstuff/ui';

import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { getForgottenSecretsMessage } from '#/settings/forgotten_secrets_message';

/** Asks for confirmation, then erases every remembered passphrase and toasts how many. */
export function useForgetSecrets(): () => Promise<void> {
	const runDaemonAction = useDaemonAction();

	return async () => {
		const isConfirmed = await ConfirmModal.call({
			title: 'Oublier les passphrases mémorisées ?',
			children:
				'Les passphrases gardées en mémoire et dans le trousseau du système seront effacées ; elles seront redemandées à la prochaine connexion.',
			confirmLabel: 'Oublier',
			cancelLabel: 'Annuler',
			closeLabel: 'Fermer',
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		await runDaemonAction(async (gateway) => {
			const { forgottenCount } = await gateway.forgetSecrets();
			useToastStore.getState().pushToast({
				tone: 'success',
				message: getForgottenSecretsMessage(forgottenCount),
			});
		}, 'Oubli impossible');
	};
}
