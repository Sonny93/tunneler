import { useToastStore } from '#/shared/toasts/toast_store';

type UseCopyDisplayCommandReturn = {
	command: string;
	copy: () => Promise<void>;
};

export function useCopyDisplayCommand(
	display: string
): UseCopyDisplayCommandReturn {
	const command = `export DISPLAY=${display}`;

	const copy = async () => {
		const { pushToast } = useToastStore.getState();
		try {
			await navigator.clipboard.writeText(command);
		} catch {
			pushToast({
				tone: 'error',
				message: 'Copie dans le presse-papiers impossible',
			});
			return;
		}
		pushToast({ tone: 'success', message: 'Commande copiée' });
	};

	return { command, copy };
}
