import { useLingui } from '@lingui/react/macro';

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
		<section className="flex flex-col gap-2">
			<h3 className="m-0 text-sm font-semibold">{t`Launchers`}</h3>
			{launchers.length === 0 ? (
				<X11LaunchersEmpty />
			) : (
				<ul role="list" className="m-0 flex list-none flex-col gap-2 p-0">
					{launchers.map((launcher) => (
						<X11LauncherItem key={launcher.name} launcher={launcher} />
					))}
				</ul>
			)}
		</section>
	);
}
