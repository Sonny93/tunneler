import { useEffect, useReducer, useState } from 'react';

import {
	toDaemonCommandError,
	type DaemonCommandError,
} from '#/daemon/daemon_command_error';
import type { ResolvedHostDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';

export type ResolvedHostState =
	| { status: 'loading' }
	| { status: 'error'; error: DaemonCommandError }
	| { status: 'ready'; resolvedHost: ResolvedHostDto };

type SettledRequest = {
	requestKey: string;
	state: Exclude<ResolvedHostState, { status: 'loading' }>;
};

const LOADING: ResolvedHostState = { status: 'loading' };

type UseResolvedHostReturn = {
	state: ResolvedHostState;
	retry: () => void;
};

/** Fetches the effective ssh configuration of a host; `retry` fetches it again. */
export function useResolvedHost(host: string): UseResolvedHostReturn {
	const gateway = useDaemonGateway();
	const [attempt, retry] = useReducer((count: number) => count + 1, 0);
	const [settled, setSettled] = useState<SettledRequest | null>(null);
	const requestKey = `${host}#${attempt}`;

	useEffect(() => {
		let isCancelled = false;
		gateway.resolveHost(host).then(
			(resolvedHost) => {
				if (isCancelled) return;
				setSettled({ requestKey, state: { status: 'ready', resolvedHost } });
			},
			(rejection: unknown) => {
				if (isCancelled) return;
				const error = toDaemonCommandError(rejection);
				setSettled({ requestKey, state: { status: 'error', error } });
			}
		);
		return () => {
			isCancelled = true;
		};
	}, [gateway, host, requestKey]);

	const state = settled?.requestKey === requestKey ? settled.state : LOADING;
	return { state, retry };
}
