import { useLingui } from '@lingui/react/macro';
import { useEffect } from 'react';

import type { DevslotLinkDto, EventMessage } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useUiStore } from '#/state/ui_store';

/**
 * Opens the dialog of the `tunneler://` link the app was started with (taken
 * once) and of every link another UI relays through `show.requested`. A link
 * that arrives while a dialog is shown is dropped and the user is told: the
 * dialog under their cursor never changes.
 */
export function useDevslotLinkListener(): void {
	const { t } = useLingui();
	const gateway = useDaemonGateway();

	useEffect(() => {
		const offer = (link: DevslotLinkDto) => {
			if (useUiStore.getState().offerDevslotLink(link)) return;
			useToastStore.getState().pushToast({
				tone: 'info',
				message: t`A Tunneler link is already waiting for your answer`,
			});
		};
		const handleEvent = (event: EventMessage) => {
			if (event.event !== 'show.requested' || event.data.link === undefined) {
				return;
			}
			offer(event.data.link);
		};

		const subscription = gateway.onEvent(handleEvent);
		gateway
			.takeLaunchLink()
			.then((link) => {
				if (link !== null) offer(link);
			})
			.catch(() => undefined);

		return () => {
			void subscription.then((unsubscribe) => unsubscribe());
		};
	}, [gateway, t]);
}
