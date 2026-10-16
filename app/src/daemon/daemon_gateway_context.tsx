import { createContext, type ReactNode } from 'react';

import type { DaemonGateway } from '#/daemon/daemon_gateway';

export const DaemonGatewayContext = createContext<DaemonGateway | null>(null);

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
