import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';
import { ConfirmModal } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { useUiStore } from '#/state/ui_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';

type UseX11AnchorActionsReturn = {
	start: () => Promise<void>;
	stop: () => Promise<void>;
};

async function confirmRisk(host: string, i18n: I18n): Promise<boolean> {
	const { confirmedX11Hosts, confirmX11Host } = useUiStore.getState();
	if (confirmedX11Hosts.includes(host)) return true;
	const isConfirmed = await ConfirmModal.call({
		title: t(i18n)`Enable X11 for ${host}?`,
		children: t(
			i18n
		)`The server ${host} will have full access to your local display (it can capture the keyboard and the screen). Only enable X11 for a trusted host.`,
		confirmLabel: t(i18n)`Enable X11`,
		confirmColor: 'danger',
	});
	if (isConfirmed) confirmX11Host(host);
	return isConfirmed;
}

/** Starting asks for confirmation the first time per host in the UI session; stopping never does. */
export function useX11AnchorActions(host: string): UseX11AnchorActionsReturn {
	const { i18n } = useLingui();
	const runDaemonAction = useDaemonAction();

	const start = async () => {
		if (!(await confirmRisk(host, i18n))) return;
		await runDaemonAction(
			(gateway) => gateway.startX11(host),
			t(i18n)`Couldn't start X11 for ${host}`
		);
	};
	const stop = async () => {
		await runDaemonAction(
			(gateway) => gateway.stopX11(host),
			t(i18n)`Couldn't stop X11 for ${host}`
		);
	};

	return { start, stop };
}
