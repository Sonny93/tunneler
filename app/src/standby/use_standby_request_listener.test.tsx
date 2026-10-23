import { ModalProvider } from '@minimalstuff/ui';
import { act, screen, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useStandbyRequestListener } from '#/standby/use_standby_request_listener';
import { useDaemonStore } from '#/state/daemon_store';
import { buildMetrics, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';

function Harness() {
	useStandbyRequestListener();
	return null;
}

const renderListener = () =>
	renderWithGateway(
		<>
			<ModalProvider />
			<Harness />
		</>
	);

describe('useStandbyRequestListener', () => {
	it('should send the awake tunnels to standby and clear the request when no connection is open', async () => {
		useDaemonStore.setState({ tunnels: [buildTunnel({ isAwake: true })] });
		const { gateway } = renderListener();

		act(() => useDaemonStore.setState({ isStandbyRequested: true }));

		await waitFor(() => expect(gateway.standbyAll).toHaveBeenCalledTimes(1));
		expect(useDaemonStore.getState().isStandbyRequested).toBe(false);
	});

	it('should ask for confirmation first when connections are open', async () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({
					isAwake: true,
					metrics: buildMetrics({ activeConnections: 2 }),
				}),
			],
		});
		const { gateway } = renderListener();

		act(() => useDaemonStore.setState({ isStandbyRequested: true }));

		expect(await screen.findByRole('alertdialog')).toHaveTextContent(
			'2 open connections will be cut.'
		);
		expect(useDaemonStore.getState().isStandbyRequested).toBe(false);
		expect(gateway.standbyAll).not.toHaveBeenCalled();
	});
});
