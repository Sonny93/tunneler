import { useLingui } from '@lingui/react/macro';
import { Card } from '@minimalstuff/ui';

import { useDaemonStore } from '#/state/daemon_store';
import { X11LauncherItem } from '#/x11/x11_launcher_item';
import { X11LaunchersEmpty } from '#/x11/x11_launchers_empty';

type X11LaunchersSectionProps = {
	host: string;
};

export function X11LaunchersSection({
	host,
}: Readonly<X11LaunchersSectionProps>) {
	const { t } = useLingui();
	const allLaunchers = useDaemonStore((state) => state.x11Launchers);
	const launchers = allLaunchers.filter((launcher) => launcher.host === host);

	return (
		<Card title={t`Launchers`} headingLevel={3} titleSize="sm" padding="sm">
			{launchers.length === 0 ? (
				<X11LaunchersEmpty />
			) : (
				<ul role="list" className="m-0 flex list-none flex-col gap-2 p-0">
					{launchers.map((launcher) => (
						<X11LauncherItem key={launcher.name} launcher={launcher} />
					))}
				</ul>
			)}
		</Card>
	);
}
