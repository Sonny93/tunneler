import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { act, screen, waitFor, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { TunnelDetailsDrawer } from '#/tunnel_details/tunnel_details_drawer';
import {
	buildMetrics,
	buildOpener,
	buildSettings,
	buildTunnel,
} from '#/testing/factories';

const LOG_NAME = 'Log of tunnel 3333 to nova:3333';

function setUp(overrides: Parameters<typeof buildTunnel>[0] = {}) {
	useDaemonStore.setState({
		tunnels: [buildTunnel(overrides)],
		settings: buildSettings(),
	});
	useUiStore.getState().openTunnelDetails('tunnel-1');
}

const renderDrawer = (gateway = createFakeGateway()) =>
	renderWithGateway(<TunnelDetailsDrawer />, gateway);

describe('TunnelDetailsDrawer', () => {
	it('should render nothing while another panel is open', () => {
		setUp();
		useUiStore.getState().openSettings();
		renderDrawer();

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should title the dialog with the tunnel endpoint', () => {
		setUp();
		renderDrawer();

		expect(
			screen.getByRole('dialog', { name: 'Tunnel 3333 → nova:3333' })
		).toBeInTheDocument();
	});

	it('should show the state and the endpoint', () => {
		setUp({ state: { kind: 'active' } });
		renderDrawer();

		const section = screen.getByRole('region', { name: 'State' });
		expect(within(section).getByText('Active')).toBeInTheDocument();
		expect(section).toHaveTextContent('3333');
	});

	it('should list the metrics', () => {
		setUp({
			metrics: buildMetrics({
				activeConnections: 2,
				totalConnections: 7,
				bytesIn: 2048,
				bytesOut: 512,
			}),
		});
		renderDrawer();

		const section = screen.getByRole('region', { name: 'Metrics' });
		expect(within(section).getByText('Active connections')).toBeInTheDocument();
		expect(section).toHaveTextContent('7');
		expect(section).toHaveTextContent('2 KB');
		expect(section).toHaveTextContent('512 B');
		expect(section).toHaveTextContent('None');
	});

	it('should update the metrics live', () => {
		setUp();
		renderDrawer();

		act(() =>
			useDaemonStore.getState().applyEvent({
				event: 'tunnel.metrics',
				data: {
					id: 'tunnel-1',
					metrics: buildMetrics({ totalConnections: 42 }),
				},
			})
		);

		expect(screen.getByRole('region', { name: 'Metrics' })).toHaveTextContent(
			'42'
		);
	});

	it('should show the equivalent ssh command', () => {
		setUp();
		renderDrawer();

		expect(
			screen.getByText('ssh -N -L 3333:127.0.0.1:3333 nova')
		).toBeInTheDocument();
	});

	it('should copy the ssh command and confirm with a toast', async () => {
		const user = userEvent.setup();
		setUp();
		renderDrawer();

		await user.click(
			screen.getByRole('button', { name: /Copy the ssh command/ })
		);

		expect(await navigator.clipboard.readText()).toBe(
			'ssh -N -L 3333:127.0.0.1:3333 nova'
		);
		expect(
			useToastStore.getState().toasts.map((toast) => toast.message)
		).toEqual(['Command copied']);
	});

	it('should ask the daemon to open a local forward', async () => {
		setUp({ state: { kind: 'active' } });
		const { gateway } = renderDrawer();

		await userEvent.click(
			screen.getByRole('button', { name: 'Open in the browser' })
		);

		expect(gateway.openTunnel).toHaveBeenCalledWith('tunnel-1', undefined);
	});

	it('should open a stopped local forward too, since the daemon starts it', async () => {
		setUp();
		const { gateway } = renderDrawer();

		await userEvent.click(
			screen.getByRole('button', { name: 'Open in the browser' })
		);

		expect(gateway.openTunnel).toHaveBeenCalledWith('tunnel-1', undefined);
	});

	it('should report an error toast when opening fails', async () => {
		setUp();
		const { gateway } = renderDrawer();
		gateway.openTunnel.mockRejectedValue(new Error('no browser'));

		await userEvent.click(
			screen.getByRole('button', { name: 'Open in the browser' })
		);

		expect(
			useToastStore.getState().toasts.map((toast) => toast.message)
		).toEqual(["Couldn't open the tunnel: no browser"]);
	});

	it('should disable restart with a visible reason when stopped', () => {
		setUp();
		renderDrawer();

		expect(
			screen.getByRole('button', { name: /Open in the browser/ })
		).toBeEnabled();
		expect(screen.getByRole('button', { name: /Restart/ })).toBeDisabled();
		expect(
			screen.getByText('Available when the tunnel is started.')
		).toBeInTheDocument();
	});

	it('should not offer the browser for a remote forward', () => {
		setUp({ kind: 'remote' });
		renderDrawer();

		expect(
			screen.queryByRole('button', { name: /Open in the browser/ })
		).not.toBeInTheDocument();
	});

	it('should not offer the target check for a dynamic forward', () => {
		setUp({ kind: 'dynamic' });
		renderDrawer();

		expect(
			screen.queryByRole('button', { name: /Check the target/ })
		).not.toBeInTheDocument();
	});

	it('should offer the target check for a remote forward', () => {
		setUp({ kind: 'remote', state: { kind: 'active' } });
		renderDrawer();

		expect(
			screen.getByRole('button', { name: 'Check the target' })
		).toBeEnabled();
	});

	it('should disable the target check with a visible reason when stopped', () => {
		setUp();
		renderDrawer();

		expect(
			screen.getByRole('button', { name: /Check the target/ })
		).toHaveAccessibleDescription('Available when the tunnel is started.');
		expect(
			screen.getByRole('button', { name: /Check the target/ })
		).toBeDisabled();
	});

	it('should check the target and announce that it is reachable', async () => {
		setUp({ state: { kind: 'active' } });
		const { gateway } = renderDrawer();

		await userEvent.click(
			screen.getByRole('button', { name: 'Check the target' })
		);

		expect(gateway.checkTarget).toHaveBeenCalledWith('tunnel-1');
		await waitFor(() =>
			expect(
				useToastStore.getState().toasts.map((toast) => toast.message)
			).toEqual(['Target reachable'])
		);
	});

	it('should announce why the target is unreachable', async () => {
		setUp({ state: { kind: 'active' } });
		const gateway = createFakeGateway();
		gateway.checkTarget.mockResolvedValue({
			kind: 'unreachable',
			since: 1,
			error: 'connection refused',
		});
		renderDrawer(gateway);

		await userEvent.click(
			screen.getByRole('button', { name: 'Check the target' })
		);

		await waitFor(() =>
			expect(
				useToastStore.getState().toasts.map((toast) => toast.message)
			).toEqual(['Target unreachable: connection refused'])
		);
	});

	it('should report the daemon error when the check fails', async () => {
		setUp({ state: { kind: 'active' } });
		const gateway = createFakeGateway();
		gateway.checkTarget.mockRejectedValue(
			new DaemonCommandError('invalid_state', 'the tunnel is stopped')
		);
		renderDrawer(gateway);

		await userEvent.click(
			screen.getByRole('button', { name: 'Check the target' })
		);

		await waitFor(() =>
			expect(
				useToastStore.getState().toasts.map((toast) => toast.message)
			).toEqual(["Couldn't check the target: the tunnel is stopped"])
		);
	});

	it('should restart a running tunnel', async () => {
		setUp({ state: { kind: 'active' } });
		const { gateway } = renderDrawer();

		await userEvent.click(screen.getByRole('button', { name: /Restart/ }));

		expect(gateway.restartTunnel).toHaveBeenCalledWith('tunnel-1');
	});

	it('should send an awake tunnel back to standby', async () => {
		setUp({ state: { kind: 'active' }, isAwake: true });
		const { gateway } = renderDrawer();

		await userEvent.click(
			screen.getByRole('button', { name: 'Send to standby' })
		);

		await waitFor(() =>
			expect(gateway.standbyTunnel).toHaveBeenCalledWith('tunnel-1')
		);
	});

	it('should not offer standby when the tunnel is not awake', () => {
		setUp({ state: { kind: 'active' } });
		renderDrawer();

		expect(
			screen.queryByRole('button', { name: 'Send to standby' })
		).not.toBeInTheDocument();
	});

	it('should open the editor', async () => {
		setUp();
		renderDrawer();

		await userEvent.click(screen.getByRole('button', { name: /Edit/ }));

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: 'tunnel-1',
		});
	});

	it('should close itself when the tunnel disappears', async () => {
		setUp();
		renderDrawer();

		act(() => useDaemonStore.setState({ tunnels: [] }));

		await waitFor(() =>
			expect(useUiStore.getState().panel).toEqual({ kind: 'none' })
		);
	});

	describe('log', () => {
		it('should show the empty state', async () => {
			setUp();
			renderDrawer();

			expect(await screen.findByText('No events recorded')).toBeInTheDocument();
		});

		it('should show the fetched entries with level text and time', async () => {
			setUp();
			const gateway = createFakeGateway();
			gateway.tunnelLogs.mockResolvedValue([
				{ at: 1_700_000_000_000, level: 'warn', message: 'retrying' },
			]);
			renderDrawer(gateway);

			const log = await screen.findByRole('log', { name: LOG_NAME });
			expect(gateway.tunnelLogs).toHaveBeenCalledWith('tunnel-1');
			expect(within(log).getByText('Warning')).toBeInTheDocument();
			expect(within(log).getByText('retrying')).toHaveAttribute('lang', 'en');
		});

		it('should be a focusable, non-live region', async () => {
			setUp();
			renderDrawer();

			const log = await screen.findByRole('log', { name: LOG_NAME });

			expect(log).toHaveAttribute('tabindex', '0');
			expect(log).toHaveAttribute('aria-live', 'off');
		});

		it('should append live entries of this tunnel only', async () => {
			setUp();
			const gateway = createFakeGateway();
			renderDrawer(gateway);
			await screen.findByRole('log', { name: LOG_NAME });

			act(() => {
				gateway.emit({
					event: 'tunnel.log',
					data: {
						id: 'tunnel-1',
						entry: { at: 1, level: 'info', message: 'hello' },
					},
				});
				gateway.emit({
					event: 'tunnel.log',
					data: {
						id: 'other',
						entry: { at: 2, level: 'error', message: 'nope' },
					},
				});
			});

			const log = screen.getByRole('log', { name: LOG_NAME });
			expect(within(log).getByText('hello')).toBeInTheDocument();
			expect(within(log).queryByText('nope')).not.toBeInTheDocument();
		});

		it('should unsubscribe on unmount', async () => {
			setUp();
			const gateway = createFakeGateway();
			const { unmount } = renderDrawer(gateway);
			await screen.findByRole('log', { name: LOG_NAME });
			unmount();

			expect(() =>
				gateway.emit({
					event: 'tunnel.log',
					data: {
						id: 'tunnel-1',
						entry: { at: 1, level: 'info', message: 'late' },
					},
				})
			).not.toThrow();
			expect(screen.queryByText('late')).not.toBeInTheDocument();
		});

		it('should show the error and retry', async () => {
			setUp();
			const gateway = createFakeGateway();
			gateway.tunnelLogs.mockRejectedValueOnce(
				new DaemonCommandError('not_found', 'tunnel not found')
			);
			renderDrawer(gateway);

			expect(await screen.findByRole('alert')).toHaveTextContent(
				'Log unavailable: tunnel not found'
			);
			await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

			expect(
				await screen.findByRole('log', { name: LOG_NAME })
			).toBeInTheDocument();
			expect(gateway.tunnelLogs).toHaveBeenCalledTimes(2);
		});
	});

	it('should have no axe violations', async () => {
		setUp({ state: { kind: 'active' } });
		const gateway = createFakeGateway();
		gateway.tunnelLogs.mockResolvedValue([
			{ at: 1_700_000_000_000, level: 'error', message: 'boom' },
		]);
		renderDrawer(gateway);
		await screen.findByRole('log', { name: LOG_NAME });

		expect(await axe(document.body)).toHaveNoViolations();
	});
});

describe('TunnelDetailsDrawer for a temporary tunnel', () => {
	it('should show the badge and swap Modifier for save and stop actions', () => {
		setUp({ isTemporary: true, state: { kind: 'active' } });
		renderDrawer();

		const actions = screen.getByRole('region', { name: 'Actions' });
		expect(screen.getAllByText('Temporary')).toHaveLength(1);
		expect(
			within(actions).queryByRole('button', { name: 'Edit' })
		).not.toBeInTheDocument();
		expect(
			within(actions).getByRole('button', { name: 'Save tunnel' })
		).toBeInTheDocument();
		expect(
			within(actions).getByRole('button', { name: 'Stop and remove' })
		).toBeInTheDocument();
	});

	it('should save the tunnel when the save action is clicked', async () => {
		setUp({ isTemporary: true });
		const { gateway } = renderDrawer();

		await userEvent.click(screen.getByRole('button', { name: 'Save tunnel' }));

		expect(gateway.persistTunnel).toHaveBeenCalledWith('tunnel-1');
	});

	it('should have no axe violations', async () => {
		setUp({ isTemporary: true });
		renderDrawer();

		expect(await axe(document.body)).toHaveNoViolations();
	});

	describe('with applications', () => {
		const vnc = buildOpener();

		it('should word the primary button after the default application', () => {
			useDaemonStore.setState({ openers: [vnc] });
			setUp({ defaultOpener: { id: 'remmina-vnc', name: 'Remmina (VNC)' } });
			renderDrawer();

			expect(
				screen.getByRole('button', { name: 'Open with Remmina (VNC)' })
			).toBeInTheDocument();
		});

		it('should offer the other choices in an "Open with…" menu', async () => {
			useDaemonStore.setState({ openers: [vnc] });
			setUp();
			const { gateway } = renderDrawer();

			await userEvent.click(screen.getByRole('button', { name: /Open with…/ }));
			await userEvent.click(
				screen.getByRole('menuitem', { name: 'Remmina (VNC)' })
			);

			expect(gateway.openTunnel).toHaveBeenCalledWith(
				'tunnel-1',
				'remmina-vnc'
			);
		});

		it('should hide the menu when there is no application', () => {
			setUp();
			renderDrawer();

			expect(
				screen.queryByRole('button', { name: /Open with…/ })
			).not.toBeInTheDocument();
		});

		it('should have no axe violations with the menu open', async () => {
			useDaemonStore.setState({ openers: [vnc] });
			setUp();
			renderDrawer();
			await userEvent.click(screen.getByRole('button', { name: /Open with…/ }));

			expect(await axe(screen.getByRole('menu'))).toHaveNoViolations();
		});
	});
});
