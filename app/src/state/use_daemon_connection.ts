import { useCallback, useEffect } from 'react';

import type { EventMessage } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import type { ConnectionState } from '#/state/daemon_state';
import { loadDaemonState } from '#/state/load_daemon_state';
import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';

type UseDaemonConnectionReturn = {
	connection: ConnectionState;
	retry: () => void;
};

async function connectAndLoad(gateway: DaemonGateway): Promise<void> {
	const { setConnection } = useDaemonStore.getState();
	setConnection({ status: 'connecting' });
	try {
		await gateway.connect();
		await loadDaemonState(gateway);
		setConnection({ status: 'connected' });
	} catch (error) {
		const { message } = toDaemonCommandError(error);
		setConnection({ status: 'disconnected', message });
	}
}

async function reloadAfterConfigChange(gateway: DaemonGateway): Promise<void> {
	try {
		await loadDaemonState(gateway);
	} catch (error) {
		const { message } = toDaemonCommandError(error);
		useDaemonStore
			.getState()
			.setConnection({ status: 'disconnected', message });
	}
}

export function useDaemonConnection(): UseDaemonConnectionReturn {
	const gateway = useDaemonGateway();
	const connection = useDaemonStore((state) => state.connection);

	useEffect(() => {
		const handleEvent = (event: EventMessage) => {
			if (event.event === 'config.reloaded') {
				void reloadAfterConfigChange(gateway);
				return;
			}
			useDaemonStore.getState().applyEvent(event);
		};
		const handleDisconnected = (message: string) =>
			useDaemonStore
				.getState()
				.setConnection({ status: 'disconnected', message });

		const subscriptions = Promise.all([
			gateway.onEvent(handleEvent),
			gateway.onDisconnected(handleDisconnected),
		]);
		let isActive = true;
		void subscriptions.then(() => {
			if (isActive) void connectAndLoad(gateway);
		});

		return () => {
			isActive = false;
			void subscriptions.then((unsubscribers) => {
				for (const unsubscribe of unsubscribers) unsubscribe();
			});
		};
	}, [gateway]);

	const retry = useCallback(() => void connectAndLoad(gateway), [gateway]);

	return { connection, retry };
}
