import { useLingui } from '@lingui/react/macro';
import { Button, ModalFooter } from '@minimalstuff/ui';
import type { RefObject } from 'react';

import { useUiStore } from '#/state/ui_store';

type DevslotLinkProblemContentProps = {
	requestId: number;
	message: string;
	/** Attached to "Close": the modal focuses it when it opens. */
	closeButtonRef: RefObject<HTMLButtonElement | null>;
};

/** What is wrong with a link that cannot be opened; nothing was done and nothing can be. */
export function DevslotLinkProblemContent({
	requestId,
	message,
	closeButtonRef,
}: Readonly<DevslotLinkProblemContentProps>) {
	const { t } = useLingui();
	const close = () => useUiStore.getState().closeDevslotLink(requestId);

	return (
		<>
			<p className="m-0">{message}</p>
			<ModalFooter>
				<Button ref={closeButtonRef} onClick={close}>
					{t`Close`}
				</Button>
			</ModalFooter>
		</>
	);
}
