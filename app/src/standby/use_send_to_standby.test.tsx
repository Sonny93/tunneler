import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { screen, waitFor } from '@testing-library/react';

import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { buildMetrics, buildTunnel } from '#/testing/factories';
import { useSendToStandby } from '#/standby/use_send_to_standby';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';

const connected = (activeConnections: number) =>
	buildMetrics({ activeConnections });

function Harness() {
	const { sendTunnel, sendTag, sendAll } = useSendToStandby();
	const [tunnel] = useDaemonStore((state) => state.tunnels);

	return (
		<>
			<button onClick={() => void sendTunnel(tunnel)}>One tunnel</button>
			<button onClick={() => void sendTag('dev')}>One tag</button>
			<button onClick={() => void sendAll()}>All</button>
		</>
	);
}

function renderHarness(gateway = createFakeGateway()) {
	return renderWithGateway(
		<>
			<ModalProvider />
			<Harness />
		</>,
		gateway
	);
}

describe('useSendToStandby', () => {
	it('should send a tunnel without confirmation when it has no open connection', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ id: 'a', isAwake: true })],
		});
		const { gateway } = renderHarness();

		await userEvent.click(screen.getByRole('button', { name: 'One tunnel' }));

		await waitFor(() =>
			expect(gateway.standbyTunnel).toHaveBeenCalledWith('a')
		);
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
	});

	it('should ask for confirmation with the connection count when a tunnel has open connections', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ id: 'a', isAwake: true, metrics: connected(2) })],
		});
		const { gateway } = renderHarness();

		await userEvent.click(screen.getByRole('button', { name: 'One tunnel' }));

		const dialog = await screen.findByRole('alertdialog', {
			name: 'Send tunnel 3333 → nova:3333 to standby?',
		});
		expect(dialog).toHaveTextContent('2 open connections will be cut.');
		expect(gateway.standbyTunnel).not.toHaveBeenCalled();
	});

	it('should send the tunnel once the confirmation is accepted', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ id: 'a', isAwake: true, metrics: connected(1) })],
		});
		const { gateway } = renderHarness();
		await userEvent.click(screen.getByRole('button', { name: 'One tunnel' }));

		await userEvent.click(
			await screen.findByRole('button', { name: 'Send to standby' })
		);

		await waitFor(() =>
			expect(gateway.standbyTunnel).toHaveBeenCalledWith('a')
		);
	});

	it('should not send the tunnel when the confirmation is cancelled', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ id: 'a', isAwake: true, metrics: connected(1) })],
		});
		const { gateway } = renderHarness();
		await userEvent.click(screen.getByRole('button', { name: 'One tunnel' }));

		await userEvent.click(
			await screen.findByRole('button', { name: 'Cancel' })
		);

		await waitFor(() =>
			expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
		);
		expect(gateway.standbyTunnel).not.toHaveBeenCalled();
	});

	it('should count only the connections of the awake tunnels of the tag', async () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({
					id: 'a',
					tags: ['dev'],
					isAwake: true,
					metrics: connected(1),
				}),
				buildTunnel({ id: 'b', tags: ['dev'], metrics: connected(5) }),
				buildTunnel({
					id: 'c',
					tags: ['db'],
					isAwake: true,
					metrics: connected(7),
				}),
			],
		});
		const { gateway } = renderHarness();

		await userEvent.click(screen.getByRole('button', { name: 'One tag' }));

		const dialog = await screen.findByRole('alertdialog', {
			name: 'Send tag dev to standby?',
		});
		expect(dialog).toHaveTextContent('1 open connection will be cut.');
		await userEvent.click(
			screen.getByRole('button', { name: 'Send to standby' })
		);
		await waitFor(() => expect(gateway.standbyTag).toHaveBeenCalledWith('dev'));
	});

	it('should send all the awake tunnels at once when none has an open connection', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ isAwake: true })],
		});
		const { gateway } = renderHarness();

		await userEvent.click(screen.getByRole('button', { name: 'All' }));

		await waitFor(() => expect(gateway.standbyAll).toHaveBeenCalledTimes(1));
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
	});

	it('should ask for confirmation before sending all the awake tunnels when connections are open', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ isAwake: true, metrics: connected(3) })],
		});
		const { gateway } = renderHarness();

		await userEvent.click(screen.getByRole('button', { name: 'All' }));

		expect(
			await screen.findByRole('alertdialog', {
				name: 'Send the awake tunnels to standby?',
			})
		).toHaveTextContent('3 open connections will be cut.');
		expect(gateway.standbyAll).not.toHaveBeenCalled();
	});

	it('should toast the failure when the daemon refuses', async () => {
		useDaemonStore.setState({ tunnels: [buildTunnel({ isAwake: true })] });
		const gateway = createFakeGateway();
		gateway.standbyAll.mockRejectedValueOnce(
			new DaemonCommandError('invalid_state', 'boom')
		);
		renderHarness(gateway);

		await userEvent.click(screen.getByRole('button', { name: 'All' }));

		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]?.message).toBe(
				"Couldn't send to standby: boom"
			)
		);
	});
});
