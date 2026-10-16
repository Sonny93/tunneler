import type { ReactNode } from 'react';
import { describe, expect, it } from 'vitest';
import { act, renderHook } from '@testing-library/react';

import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_context';

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
			run((gateway) => gateway.startTunnel('a'), 'Démarrage impossible')
		);

		expect(isSuccess).toBe(true);
	});

	it('should push an error toast with the daemon message when the call fails', async () => {
		const { gateway, run } = renderDaemonAction();
		gateway.startTunnel.mockRejectedValueOnce({
			code: 'not_found',
			message: 'tunnel inconnu',
		});

		await act(() =>
			run((daemon) => daemon.startTunnel('a'), 'Démarrage impossible')
		);

		expect(useToastStore.getState().toasts).toEqual([
			expect.objectContaining({
				tone: 'error',
				message: 'Démarrage impossible : tunnel inconnu',
			}),
		]);
	});
});
