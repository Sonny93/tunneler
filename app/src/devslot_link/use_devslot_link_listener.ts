import { useLingui } from '@lingui/react/macro';
import { useEffect } from 'react';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import type { DevslotLinkDto, EventMessage } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { pushToast } from '#/shared/toasts/push_toast';
import { useUiStore } from '#/state/ui_store';

/**
 * Opens the dialog of the `tunneler://` link the app was started with (taken
 * once) and of every link another UI relays through `show.requested`. A link
 * that arrives while a dialog is shown is dropped and the user is told: the
 * dialog under their cursor never changes.
 */
export function useDevslotLinkListener(): void {
	const { t, i18n } = useLingui();
	const gateway = useDaemonGateway();

	useEffect(() => {
		const offer = (link: DevslotLinkDto) => {
			if (useUiStore.getState().offerDevslotLink(link)) return;
			pushToast({
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
			.catch((rejection: unknown) => {
				const description = describeDaemonError(
					toDaemonCommandError(rejection),
					i18n
				);
				pushToast({
					tone: 'error',
					message: t`Couldn't read the link this app was started with: ${description}`,
				});
			});

		return () => {
			void subscription.then((unsubscribe) => unsubscribe());
		};
	}, [gateway, i18n, t]);
}
