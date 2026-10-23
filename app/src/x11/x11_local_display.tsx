import { Trans } from '@lingui/react/macro';

import { useDaemonStore } from '#/state/daemon_store';

type X11LocalDisplayProps = {
	host: string;
};

export function X11LocalDisplay({ host }: Readonly<X11LocalDisplayProps>) {
	const hostDisplay = useDaemonStore((state) => state.x11HostDisplays[host]);
	const settings = useDaemonStore((state) => state.settings);
	if (settings === null || settings === undefined) return null;

	const settingsDisplay = settings.x11Display;

	return (
		<p className="m-0 text-sm text-tn-muted">
			{hostDisplay ? (
				<Trans>
					Local display: <span className="font-mono">{hostDisplay}</span> (this
					host, in tunnels.toml)
				</Trans>
			) : settingsDisplay ? (
				<Trans>
					Local display: <span className="font-mono">{settingsDisplay}</span>{' '}
					(general setting)
				</Trans>
			) : (
				<Trans>
					Local display: DISPLAY variable (127.0.0.1:0.0 on Windows) (general
					setting)
				</Trans>
			)}
		</p>
	);
}
