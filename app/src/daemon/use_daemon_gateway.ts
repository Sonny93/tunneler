import { useContext } from 'react';

import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { DaemonGatewayContext } from '#/daemon/daemon_gateway_context';
import { MissingDaemonGatewayError } from '#/daemon/missing_daemon_gateway_error';

export function useDaemonGateway(): DaemonGateway {
	const gateway = useContext(DaemonGatewayContext);
	if (!gateway) throw new MissingDaemonGatewayError();
	return gateway;
}
