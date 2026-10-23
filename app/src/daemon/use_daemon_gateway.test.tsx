import { render, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_provider';
import { MissingDaemonGatewayError } from '#/daemon/missing_daemon_gateway_error';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { createFakeGateway } from '#/testing/fake_gateway';

describe('useDaemonGateway', () => {
	it('should return the gateway given to the provider', () => {
		const gateway = createFakeGateway();

		const { result } = renderHook(() => useDaemonGateway(), {
			wrapper: ({ children }) => (
				<DaemonGatewayProvider gateway={gateway}>
					{children}
				</DaemonGatewayProvider>
			),
		});

		expect(result.current).toBe(gateway);
	});

	it('should throw a MissingDaemonGatewayError outside the provider', () => {
		vi.spyOn(console, 'error').mockImplementation(() => undefined);
		const Consumer = () => {
			useDaemonGateway();
			return null;
		};

		expect(() => render(<Consumer />)).toThrow(MissingDaemonGatewayError);
	});
});
