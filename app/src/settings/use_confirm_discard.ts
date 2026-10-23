import { useLingui } from '@lingui/react/macro';
import { ConfirmModal, useDismissGuard } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';

type UseConfirmDiscardReturn = {
	/**
	 * Cancel button path: closes the settings at once when clean, otherwise
	 * asks first and closes the panel once the user agreed to lose their changes.
	 */
	confirmDiscard: () => Promise<void>;
};

/**
 * Guards every way of leaving the settings while the form is dirty.
 * Escape, the backdrop and the close button go through a dismiss guard of the
 * modal, which ends the call itself when the user discards. The Cancel button
 * asks through the same confirmation and closes the panel explicitly.
 */
export function useConfirmDiscard(isDirty: boolean): UseConfirmDiscardReturn {
	const { t } = useLingui();

	const askToDiscard = () =>
		ConfirmModal.call({
			title: t`Discard your changes?`,
			children: t`The changes you made to the settings are not saved yet.`,
			confirmLabel: t`Discard`,
			cancelLabel: t`Keep editing`,
			confirmColor: 'danger',
		});

	useDismissGuard(askToDiscard, isDirty);

	const confirmDiscard = async () => {
		const isDiscarded = !isDirty || (await askToDiscard());
		if (isDiscarded) useUiStore.getState().closePanel();
	};

	return { confirmDiscard };
}
