import { createContext } from 'react';

import type { DaemonGateway } from '#/daemon/daemon_gateway';

export const DaemonGatewayContext = createContext<DaemonGateway | null>(null);
