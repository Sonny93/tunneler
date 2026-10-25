import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_provider';
import type { X11ProbeDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';
import { createFakeGateway, type FakeGateway } from '#/testing/fake_gateway';
import { I18nTestProvider } from '#/testing/i18n_test_provider';
import {
	useX11ServerStatus,
	X11_SERVER_POLL_INTERVAL_MS,
} from '#/x11/use_x11_server_status';

const reachable: X11ProbeDto = {
	display: ':0',
	isReachable: true,
	server: null,
};
const stopped: X11ProbeDto = {
	display: '127.0.0.1:0.0',
	isReachable: false,
	server: 'VcXsrv',
};

function setup(isActive = true, probe: Promise<X11ProbeDto> | null = null) {
	const gateway: FakeGateway = createFakeGateway();
	gateway.probeX11.mockReturnValue(probe ?? Promise.resolve(reachable));
	const wrapper = ({ children }: Readonly<{ children: ReactNode }>) => (
		<I18nTestProvider>
			<DaemonGatewayProvider gateway={gateway}>
				{children}
			</DaemonGatewayProvider>
		</I18nTestProvider>
	);
	const rendered = renderHook(
		({ isActive: isHookActive }) => useX11ServerStatus('nova', isHookActive),
		{ wrapper, initialProps: { isActive } }
	);
	return { gateway, ...rendered };
}

const advance = (milliseconds: number) =>
	act(() => vi.advanceTimersByTimeAsync(milliseconds));

describe('useX11ServerStatus', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('should be checking until the first probe answers', async () => {
		const { result, gateway } = setup();

		expect(result.current.status).toEqual({ kind: 'checking' });
		await advance(0);

		expect(gateway.probeX11).toHaveBeenCalledWith('nova');
		expect(result.current.status).toEqual({
			kind: 'reachable',
			display: ':0',
		});
	});

	it('should probe again every interval and follow the answer', async () => {
		const { result, gateway } = setup();
		await advance(0);
		gateway.probeX11.mockResolvedValue(stopped);

		await advance(X11_SERVER_POLL_INTERVAL_MS);

		expect(gateway.probeX11).toHaveBeenCalledTimes(2);
		expect(result.current.status).toEqual({
			kind: 'unreachable',
			display: '127.0.0.1:0.0',
			server: 'VcXsrv',
		});
	});

	it('should keep the same status object when a probe finds the same thing', async () => {
		const { result } = setup();
		await advance(0);
		const before = result.current.status;

		await advance(X11_SERVER_POLL_INTERVAL_MS);

		expect(result.current.status).toBe(before);
	});

	it('should skip a tick while a probe is still in flight', async () => {
		const { gateway } = setup(true, new Promise(() => {}));

		await advance(X11_SERVER_POLL_INTERVAL_MS * 3);

		expect(gateway.probeX11).toHaveBeenCalledTimes(1);
	});

	it('should report a failed probe with the error message', async () => {
		const { result, gateway } = setup();
		gateway.probeX11.mockRejectedValue(
			new DaemonCommandError('x11_server', 'no display')
		);

		await advance(X11_SERVER_POLL_INTERVAL_MS);

		expect(result.current.status).toEqual({
			kind: 'failed',
			reason: 'no display',
		});
	});

	it('should stop probing when unmounted', async () => {
		const { gateway, unmount } = setup();
		await advance(0);

		unmount();
		await advance(X11_SERVER_POLL_INTERVAL_MS * 3);

		expect(gateway.probeX11).toHaveBeenCalledTimes(1);
	});

	it('should not probe while inactive', async () => {
		const { gateway } = setup(false);

		await advance(X11_SERVER_POLL_INTERVAL_MS * 2);

		expect(gateway.probeX11).not.toHaveBeenCalled();
	});

	it('should start the server then probe again at once', async () => {
		const { result, gateway } = setup();
		gateway.probeX11.mockResolvedValue(stopped);
		await advance(0);
		gateway.probeX11.mockResolvedValue(reachable);

		await act(() => result.current.startServer('VcXsrv'));

		expect(gateway.startX11Server).toHaveBeenCalledWith('nova');
		expect(gateway.probeX11).toHaveBeenCalledTimes(2);
		expect(result.current.status.kind).toBe('reachable');
		expect(result.current.isStarting).toBe(false);
	});

	it('should toast and probe again when the server fails to start', async () => {
		const { result, gateway } = setup();
		gateway.startX11Server.mockRejectedValue(
			new DaemonCommandError('x11_server', 'not found')
		);
		await advance(0);

		await act(() => result.current.startServer('VcXsrv'));

		expect(
			useToastStore.getState().toasts.map((toast) => toast.message)
		).toEqual(["Couldn't start VcXsrv: not found"]);
		expect(gateway.probeX11).toHaveBeenCalledTimes(2);
	});
});
