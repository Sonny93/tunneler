import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it } from 'vitest';

import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_context';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { useToastStore } from '#/shared/toasts/toast_store';
import { createFakeGateway } from '#/testing/fake_gateway';

function renderDaemonAction() {
	const gateway = createFakeGateway();
	const wrapper = ({ children }: Readonly<{ children: ReactNode }>) => (
		<DaemonGatewayProvider gateway={gateway}>{children}</DaemonGatewayProvider>
	);
	const { result } = renderHook(() => useDaemonAction(), { wrapper });
	return { gateway, run: result.current };
}

describe('useDaemonAction', () => {
	it('should resolve to true when the call succeeds', async () => {
		const { run } = renderDaemonAction();

		const isSuccess = await act(() =>
			run((gateway) => gateway.startTunnel('a'), 'Could not start')
		);

		expect(isSuccess).toBe(true);
	});

	it('should push an error toast with the daemon message when the call fails', async () => {
		const { gateway, run } = renderDaemonAction();
		gateway.startTunnel.mockRejectedValueOnce({
			code: 'not_found',
			message: 'unknown tunnel',
		});

		await act(() =>
			run((daemon) => daemon.startTunnel('a'), 'Could not start')
		);

		expect(useToastStore.getState().toasts).toEqual([
			expect.objectContaining({
				tone: 'error',
				message: 'Could not start: unknown tunnel',
			}),
		]);
	});

	it('should push a translated message when the error code is mapped', async () => {
		const { gateway, run } = renderDaemonAction();
		gateway.startTunnel.mockRejectedValueOnce({
			code: 'disconnected',
			message: 'raw bridge text',
		});

		await act(() =>
			run((daemon) => daemon.startTunnel('a'), 'Could not start')
		);

		expect(useToastStore.getState().toasts).toEqual([
			expect.objectContaining({
				message: 'Could not start: The daemon is not reachable',
			}),
		]);
	});
});
