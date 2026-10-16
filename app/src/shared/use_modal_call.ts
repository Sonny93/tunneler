import { useEffect, useRef } from 'react';
import { Modal, type ModalProps } from '@minimalstuff/ui';

export type ModalCallRequest = {
	id: string;
	props: ModalProps;
};

/**
 * Keeps a lib `Modal` open while `request` is set (a `<ModalProvider />` must
 * already be mounted). The modal is tied to `request.id`: changing or removing
 * the request ends it. `onDismissed` fires only when the user closed the modal
 * themselves (Escape, close button, a `close()` call) while the request was
 * still current.
 */
export function useModalCall(
	request: Readonly<ModalCallRequest> | null,
	onDismissed: (id: string) => void
): void {
	const latest = useRef({ request, onDismissed });
	useEffect(() => {
		latest.current = { request, onDismissed };
	});

	const requestId = request?.id ?? null;
	useEffect(() => {
		const current = latest.current.request;
		if (requestId === null || current === null) return;
		let isReleased = false;
		const call = Modal.call(current.props);
		void call.then(() => {
			if (!isReleased) latest.current.onDismissed(requestId);
		});
		return () => {
			isReleased = true;
			Modal.end(call, undefined);
		};
	}, [requestId]);
}
