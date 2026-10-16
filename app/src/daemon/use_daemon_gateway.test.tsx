import { describe, expect, it, vi } from 'vitest';
import { render, renderHook } from '@testing-library/react';

import { createFakeGateway } from '#/testing/fake_gateway';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_context';
import { MissingDaemonGatewayError } from '#/daemon/missing_daemon_gateway_error';

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
