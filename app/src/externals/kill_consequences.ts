import { msg } from '@lingui/core/macro';

export const KILL_EXTERNAL_CONSEQUENCE = msg`All the tunnels this process provides stop as well.`;

export const KILL_CONFLICT_CONSEQUENCE = msg`The port is freed: the tunnel can then be started.`;
