import { t } from '@lingui/core/macro';

import { pushToast } from '#/shared/toasts/push_toast';

/** Copies `text` to the clipboard and reports the outcome as a toast. */
export async function copyTextWithToast(
	text: string,
	successMessage: string
): Promise<void> {
	try {
		await navigator.clipboard.writeText(text);
	} catch {
		pushToast({
			tone: 'error',
			message: t`Couldn't copy to the clipboard`,
		});
		return;
	}
	pushToast({ tone: 'success', message: successMessage });
}
