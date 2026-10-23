import type { ReactNode } from 'react';

import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { DaemonGatewayContext } from '#/daemon/daemon_gateway_context';

type DaemonGatewayProviderProps = {
	gateway: DaemonGateway;
	children: ReactNode;
};

export const DaemonGatewayProvider = ({
	gateway,
	children,
}: Readonly<DaemonGatewayProviderProps>) => (
	<DaemonGatewayContext value={gateway}>{children}</DaemonGatewayContext>
);
