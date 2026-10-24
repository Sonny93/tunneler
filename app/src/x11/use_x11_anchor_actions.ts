import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { ConfirmModal } from '@minimalstuff/ui';

import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useUiStore } from '#/state/ui_store';
import { useEnsureX11Server } from '#/x11/use_ensure_x11_server';

type UseX11AnchorActionsReturn = {
	start: () => Promise<void>;
	stop: () => Promise<void>;
};

async function confirmRisk(host: string, i18n: I18n): Promise<boolean> {
	const { confirmedX11Hosts, confirmX11Host } = useUiStore.getState();
	if (confirmedX11Hosts.includes(host)) return true;
	const isConfirmed = await ConfirmModal.call({
		title: i18n._(msg`Enable X11 for ${host}?`),
		children: i18n._(
			msg`The server ${host} will have full access to your local display (it can capture the keyboard and the screen). Only enable X11 for a trusted host.`
		),
		confirmLabel: i18n._(msg`Enable X11`),
		confirmColor: 'danger',
	});
	if (isConfirmed) confirmX11Host(host);
	return isConfirmed;
}

/** Starting asks for confirmation the first time per host in the UI session; stopping never does. */
export function useX11AnchorActions(host: string): UseX11AnchorActionsReturn {
	const { i18n } = useLingui();
	const runDaemonAction = useDaemonAction();
	const ensureX11Server = useEnsureX11Server();

	const start = async () => {
		if (!(await confirmRisk(host, i18n))) return;
		await ensureX11Server(host);
		await runDaemonAction(
			(gateway) => gateway.startX11(host),
			i18n._(msg`Couldn't start X11 for ${host}`)
		);
	};
	const stop = async () => {
		await runDaemonAction(
			(gateway) => gateway.stopX11(host),
			i18n._(msg`Couldn't stop X11 for ${host}`)
		);
	};

	return { start, stop };
}
