import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { ConfirmModal } from '@minimalstuff/ui';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import type { X11ProbeDto } from '#/daemon/protocol';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';
import {
	decideX11ServerAction,
	type X11ServerAction,
} from '#/x11/decide_x11_server_action';

const DEFAULT_SETTING = 'ask';

/**
 * Checks that an X server answers on the local display and, depending on the
 * `x11ServerStart` setting, starts the installed one, asks, or warns. Never
 * blocks: it always resolves so the caller goes on with X11.
 */
export function useEnsureX11Server(): (host: string) => Promise<void> {
	const { i18n } = useLingui();
	const gateway = useDaemonGateway();
	const runDaemonAction = useDaemonAction();

	const warn = (message: string) =>
		useToastStore.getState().pushToast({ tone: 'warning', message });

	const probe = async (host: string): Promise<X11ProbeDto | null> => {
		try {
			return await gateway.probeX11(host);
		} catch (rejection) {
			const reason = describeDaemonError(toDaemonCommandError(rejection), i18n);
			warn(i18n._(msg`Couldn't check the X server: ${reason}`));
			return null;
		}
	};

	const startServer = async (host: string, server: string) => {
		await runDaemonAction(
			(daemonGateway) => daemonGateway.startX11Server(host),
			i18n._(msg`Couldn't start ${server}`)
		);
	};

	const askToStart = (host: string, display: string, server: string) =>
		ConfirmModal.call({
			title: i18n._(msg`Start ${server}?`),
			children: i18n._(
				msg`No X server is running on ${display}, so windows opened on ${host} would not appear.`
			),
			confirmLabel: i18n._(msg`Start ${server}`),
			cancelLabel: i18n._(msg`Continue without`),
		});

	const act = async (
		host: string,
		display: string,
		action: X11ServerAction
	) => {
		switch (action.kind) {
			case 'proceed':
				return;
			case 'start':
				await startServer(host, action.server);
				return;
			case 'ask':
				if (await askToStart(host, display, action.server)) {
					await startServer(host, action.server);
				}
				return;
			case 'warnNotRunning':
				warn(
					i18n._(
						msg`No X server is running on ${display}: windows opened on ${host} will not appear.`
					)
				);
				return;
			case 'warnMissing':
				warn(
					i18n._(
						msg`No X server is running on ${display}: windows opened on ${host} will not appear. Start your X server, or on Windows install one such as VcXsrv.`
					)
				);
		}
	};

	return async (host) => {
		const found = await probe(host);
		if (found === null) return;
		const setting =
			useDaemonStore.getState().settings?.x11ServerStart ?? DEFAULT_SETTING;
		await act(host, found.display, decideX11ServerAction(found, setting));
	};
}
