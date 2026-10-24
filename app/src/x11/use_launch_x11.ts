import { useLingui } from '@lingui/react/macro';

import type { X11LauncherDto } from '#/daemon/protocol';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useEnsureX11Server } from '#/x11/use_ensure_x11_server';

export function useLaunchX11(): (
	launcher: Readonly<X11LauncherDto>
) => Promise<void> {
	const { t } = useLingui();
	const runDaemonAction = useDaemonAction();
	const ensureX11Server = useEnsureX11Server();

	return async ({ host, name }) => {
		await ensureX11Server(host);
		const isLaunched = await runDaemonAction(
			(gateway) => gateway.launchX11(host, name),
			t`Couldn't launch ${name} on ${host}`
		);
		if (!isLaunched) return;
		useToastStore
			.getState()
			.pushToast({ tone: 'success', message: t`${name} launched on ${host}` });
	};
}
