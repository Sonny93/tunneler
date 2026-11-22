import { useLingui } from '@lingui/react/macro';
import { Card } from '@minimalstuff/ui';

import { useX11ServerStatus } from '#/x11/use_x11_server_status';
import { X11ServerStatus } from '#/x11/x11_server_status';

type X11ServerSectionProps = {
	host: string;
	isOpen: boolean;
};

export function X11ServerSection({
	host,
	isOpen,
}: Readonly<X11ServerSectionProps>) {
	const { t } = useLingui();
	const { status, isStarting, startServer } = useX11ServerStatus(host, isOpen);

	return (
		<Card title={t`X server`} headingLevel={3} padding="sm">
			<X11ServerStatus
				status={status}
				isStarting={isStarting}
				onStart={(server) => void startServer(server)}
			/>
		</Card>
	);
}
