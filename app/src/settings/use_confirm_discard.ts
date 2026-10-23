import { useLingui } from '@lingui/react/macro';
import { ConfirmModal } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';

type UseConfirmDiscardReturn = {
	/** Closes the settings at once, or after the user agreed to lose their changes. */
	confirmDiscard: () => Promise<void>;
};

export function useConfirmDiscard(isDirty: boolean): UseConfirmDiscardReturn {
	const { t } = useLingui();

	const confirmDiscard = async () => {
		const isDiscarded =
			!isDirty ||
			(await ConfirmModal.call({
				title: t`Discard your changes?`,
				children: t`The changes you made to the settings are not saved yet.`,
				confirmLabel: t`Discard`,
				cancelLabel: t`Keep editing`,
				confirmColor: 'danger',
			}));
		if (isDiscarded) useUiStore.getState().closePanel();
	};

	return { confirmDiscard };
}
