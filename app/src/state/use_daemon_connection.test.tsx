import { act, renderHook, waitFor } from '@testing-library/react';
import { StrictMode, type ReactNode } from 'react';
import { describe, expect, it } from 'vitest';

import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_context';
import { useDaemonStore } from '#/state/daemon_store';
import { useDaemonConnection } from '#/state/use_daemon_connection';
import { buildSnapshot, buildTunnel } from '#/testing/factories';
import { createFakeGateway, type FakeGateway } from '#/testing/fake_gateway';

const renderConnection = (gateway: FakeGateway, isStrict = false) => {
	const wrapper = ({ children }: { children: ReactNode }) => {
		const provider = (
			<DaemonGatewayProvider gateway={gateway}>
				{children}
			</DaemonGatewayProvider>
		);
		return isStrict ? <StrictMode>{provider}</StrictMode> : provider;
	};
	return renderHook(() => useDaemonConnection(), { wrapper });
};

describe('useDaemonConnection', () => {
	it('should be connecting before the daemon answers', () => {
		const gateway = createFakeGateway();
		gateway.connect.mockReturnValue(new Promise(() => undefined));

		const { result } = renderConnection(gateway);

		expect(result.current.connection).toEqual({ status: 'connecting' });
	});

	it('should load the snapshot and the hosts once connected', async () => {
		const gateway = createFakeGateway({ hosts: ['nova'] });

		const { result } = renderConnection(gateway);

		await waitFor(() =>
			expect(result.current.connection.status).toBe('connected')
		);
		expect(useDaemonStore.getState().hosts).toEqual(['nova']);
		expect(useDaemonStore.getState().tunnels).toHaveLength(1);
	});

	it('should subscribe to events before connecting', async () => {
		const gateway = createFakeGateway();

		renderConnection(gateway);

		await waitFor(() => expect(gateway.connect).toHaveBeenCalled());
		expect(gateway.onEvent.mock.invocationCallOrder[0]).toBeLessThan(
			gateway.connect.mock.invocationCallOrder[0] ?? 0
		);
	});

	it('should keep the app usable with no hosts when listHosts fails', async () => {
		const gateway = createFakeGateway();
		gateway.listHosts.mockRejectedValue(new Error('no ssh config'));

		const { result } = renderConnection(gateway);

		await waitFor(() =>
			expect(result.current.connection.status).toBe('connected')
		);
		expect(useDaemonStore.getState().hosts).toEqual([]);
	});

	it('should be disconnected with the error when connect fails', async () => {
		const gateway = createFakeGateway();
		gateway.connect.mockRejectedValue({
			code: 'daemon_unreachable',
			message: 'not found',
		});

		const { result } = renderConnection(gateway);

		await waitFor(() =>
			expect(result.current.connection).toEqual({
				status: 'disconnected',
				error: expect.objectContaining({
					code: 'daemon_unreachable',
					message: 'not found',
				}),
			})
		);
	});

	it('should connect again on retry', async () => {
		const gateway = createFakeGateway();
		gateway.connect.mockRejectedValueOnce({
			code: 'daemon_unreachable',
			message: 'x',
		});
		const { result } = renderConnection(gateway);
		await waitFor(() =>
			expect(result.current.connection.status).toBe('disconnected')
		);

		act(() => result.current.retry());

		await waitFor(() =>
			expect(result.current.connection.status).toBe('connected')
		);
		expect(gateway.connect).toHaveBeenCalledTimes(2);
	});

	it('should become disconnected when the gateway reports a disconnection', async () => {
		const gateway = createFakeGateway();
		const { result } = renderConnection(gateway);
		await waitFor(() =>
			expect(result.current.connection.status).toBe('connected')
		);

		act(() => gateway.disconnect('lost'));

		expect(result.current.connection).toEqual({
			status: 'disconnected',
			error: expect.objectContaining({ message: 'lost' }),
		});
	});

	it('should apply pushed events to the store', async () => {
		const gateway = createFakeGateway({
			snapshot: buildSnapshot({ tunnels: [buildTunnel({ id: 'a' })] }),
		});
		const { result } = renderConnection(gateway);
		await waitFor(() =>
			expect(result.current.connection.status).toBe('connected')
		);

		act(() =>
			gateway.emit({
				event: 'tunnel.changed',
				data: {
					id: 'a',
					state: { kind: 'active' },
					health: { kind: 'unknown' },
					isAwake: false,
				},
			})
		);

		expect(useDaemonStore.getState().tunnels[0]?.state).toEqual({
			kind: 'active',
		});
	});

	it('should reload the snapshot and clear errors on config.reloaded', async () => {
		const gateway = createFakeGateway();
		const { result } = renderConnection(gateway);
		await waitFor(() =>
			expect(result.current.connection.status).toBe('connected')
		);
		act(() =>
			gateway.emit({ event: 'config.invalid', data: { messages: ['bad'] } })
		);

		act(() => gateway.emit({ event: 'config.reloaded', data: {} }));

		await waitFor(() =>
			expect(useDaemonStore.getState().configErrors).toEqual([])
		);
		expect(gateway.snapshot).toHaveBeenCalledTimes(2);
	});

	it('should unsubscribe both listeners on unmount', async () => {
		const gateway = createFakeGateway();
		const { result, unmount } = renderConnection(gateway);
		await waitFor(() =>
			expect(result.current.connection.status).toBe('connected')
		);

		unmount();
		await new Promise((resolve) => setTimeout(resolve, 0));
		gateway.emit({ event: 'quit.requested', data: {} });

		expect(useDaemonStore.getState().isQuitRequested).toBe(false);
	});

	it('should apply each event once under StrictMode', async () => {
		const gateway = createFakeGateway({
			snapshot: buildSnapshot({ tunnels: [] }),
		});
		const { result } = renderConnection(gateway, true);
		await waitFor(() =>
			expect(result.current.connection.status).toBe('connected')
		);
		act(() =>
			gateway.emit({
				event: 'credential.request',
				data: { promptId: 'p1', kind: 'passphrase', alias: 'a', keyPath: '/k' },
			})
		);

		expect(useDaemonStore.getState().prompts).toHaveLength(1);
	});
});
