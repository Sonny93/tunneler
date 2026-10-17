import { useToastStore } from '#/shared/toasts/toast_store';

/** Copies `text` to the clipboard and reports the outcome as a toast. */
export async function copyTextWithToast(
	text: string,
	successMessage: string
): Promise<void> {
	const { pushToast } = useToastStore.getState();
	try {
		await navigator.clipboard.writeText(text);
	} catch {
		pushToast({
			tone: 'error',
			message: 'Copie dans le presse-papiers impossible',
		});
		return;
	}
	pushToast({ tone: 'success', message: successMessage });
}
