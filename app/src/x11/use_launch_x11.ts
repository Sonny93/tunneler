import type { X11LauncherDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';

export function useLaunchX11(): (
	launcher: Readonly<X11LauncherDto>
) => Promise<void> {
	const runDaemonAction = useDaemonAction();

	return async ({ host, name }) => {
		const isLaunched = await runDaemonAction(
			(gateway) => gateway.launchX11(host, name),
			`Lancement de ${name} sur ${host} impossible`
		);
		if (!isLaunched) return;
		useToastStore
			.getState()
			.pushToast({ tone: 'success', message: `${name} lancé sur ${host}` });
	};
}
