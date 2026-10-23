import { useEffect, useReducer } from 'react';

import {
	toDaemonCommandError,
	type DaemonCommandError,
} from '#/daemon/daemon_command_error';
import type { LogEntryDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { appendLogEntry, mergeLogEntries } from '#/tunnel_details/log_entries';

export type TunnelLogsState = {
	status: 'loading' | 'ready' | 'error';
	entries: readonly LogEntryDto[];
	error: DaemonCommandError | null;
};

type TunnelLogsAction =
	| { type: 'fetchStarted' }
	| { type: 'fetchSucceeded'; fetched: readonly LogEntryDto[] }
	| { type: 'fetchFailed'; error: DaemonCommandError }
	| { type: 'entryReceived'; entry: LogEntryDto };

const INITIAL_STATE: TunnelLogsState = {
	status: 'loading',
	entries: [],
	error: null,
};

function reduceTunnelLogs(
	state: TunnelLogsState,
	action: TunnelLogsAction
): TunnelLogsState {
	switch (action.type) {
		case 'fetchStarted':
			return { ...state, status: 'loading', error: null };
		case 'fetchSucceeded':
			return {
				status: 'ready',
				entries: mergeLogEntries(action.fetched, state.entries),
				error: null,
			};
		case 'fetchFailed':
			return { ...state, status: 'error', error: action.error };
		case 'entryReceived':
			return {
				...state,
				entries: appendLogEntry(state.entries, action.entry),
			};
	}
}

type UseTunnelLogsReturn = {
	state: TunnelLogsState;
	retry: () => void;
};

/** The history of a tunnel followed by its live log events; entries received before the history arrives are merged in. */
export function useTunnelLogs(tunnelId: string): UseTunnelLogsReturn {
	const gateway = useDaemonGateway();
	const [state, dispatch] = useReducer(reduceTunnelLogs, INITIAL_STATE);
	const [attempt, retry] = useReducer((count: number) => count + 1, 0);

	useEffect(() => {
		const subscription = gateway.onEvent((event) => {
			if (event.event !== 'tunnel.log' || event.data.id !== tunnelId) return;
			dispatch({ type: 'entryReceived', entry: event.data.entry });
		});
		return () => void subscription.then((unsubscribe) => unsubscribe());
	}, [gateway, tunnelId]);

	useEffect(() => {
		let isCancelled = false;
		dispatch({ type: 'fetchStarted' });
		gateway.tunnelLogs(tunnelId).then(
			(fetched) => {
				if (!isCancelled) dispatch({ type: 'fetchSucceeded', fetched });
			},
			(rejection: unknown) => {
				if (isCancelled) return;
				dispatch({
					type: 'fetchFailed',
					error: toDaemonCommandError(rejection),
				});
			}
		);
		return () => {
			isCancelled = true;
		};
	}, [gateway, tunnelId, attempt]);

	return { state, retry };
}
