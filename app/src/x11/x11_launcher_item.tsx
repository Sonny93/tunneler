import { Button } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { useLaunchX11 } from '#/x11/use_launch_x11';
import type { X11LauncherDto } from '#/daemon/protocol';

type X11LauncherItemProps = {
	launcher: X11LauncherDto;
};

export function X11LauncherItem({ launcher }: Readonly<X11LauncherItemProps>) {
	const { t } = useLingui();
	const launch = useLaunchX11();
	const { name, host } = launcher;

	const handleLaunch = () => void launch(launcher);

	return (
		<li className="flex items-center justify-between gap-2 rounded-md border border-tn-divider px-3 py-2">
			<div className="flex min-w-0 flex-col">
				<span className="font-semibold">{launcher.name}</span>
				<code className="font-mono text-xs break-all text-tn-muted">
					{launcher.command}
				</code>
			</div>
			<Button
				size="sm"
				startIcon="i-mdi-rocket-launch-outline"
				aria-label={t`Launch ${name} on ${host}`}
				onClick={handleLaunch}
			>
				{t`Launch`}
			</Button>
		</li>
	);
}
