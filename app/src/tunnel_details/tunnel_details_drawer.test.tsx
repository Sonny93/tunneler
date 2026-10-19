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
import { buildMetrics, buildSettings, buildTunnel } from '#/testing/factories';

const LOG_NAME = 'Journal du tunnel 3333 vers nova:3333';

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

		const section = screen.getByRole('region', { name: 'État' });
		expect(within(section).getByText('Actif')).toBeInTheDocument();
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

		const section = screen.getByRole('region', { name: 'Métriques' });
		expect(within(section).getByText('Connexions actives')).toBeInTheDocument();
		expect(section).toHaveTextContent('7');
		expect(section).toHaveTextContent('2 Ko');
		expect(section).toHaveTextContent('512 o');
		expect(section).toHaveTextContent('Aucune');
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

		expect(screen.getByRole('region', { name: 'Métriques' })).toHaveTextContent(
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
			screen.getByRole('button', { name: /Copier la commande ssh/ })
		);

		expect(await navigator.clipboard.readText()).toBe(
			'ssh -N -L 3333:127.0.0.1:3333 nova'
		);
		expect(
			useToastStore.getState().toasts.map((toast) => toast.message)
		).toEqual(['Commande copiée']);
	});

	it('should open a running local forward in the browser', async () => {
		setUp({ state: { kind: 'active' } });
		const { gateway } = renderDrawer();

		await userEvent.click(
			screen.getByRole('button', { name: 'Ouvrir dans le navigateur' })
		);

		expect(gateway.openInBrowser).toHaveBeenCalledWith('127.0.0.1', 3333);
	});

	it('should disable the browser and restart with a visible reason when stopped', () => {
		setUp();
		renderDrawer();

		expect(
			screen.getByRole('button', { name: /Ouvrir dans le navigateur/ })
		).toBeDisabled();
		expect(screen.getByRole('button', { name: /Redémarrer/ })).toBeDisabled();
		expect(
			screen.getByText('Disponible quand le tunnel est démarré.')
		).toBeInTheDocument();
	});

	it('should not offer the browser for a remote forward', () => {
		setUp({ kind: 'remote' });
		renderDrawer();

		expect(
			screen.queryByRole('button', { name: /Ouvrir dans le navigateur/ })
		).not.toBeInTheDocument();
	});

	it('should restart a running tunnel', async () => {
		setUp({ state: { kind: 'active' } });
		const { gateway } = renderDrawer();

		await userEvent.click(screen.getByRole('button', { name: /Redémarrer/ }));

		expect(gateway.restartTunnel).toHaveBeenCalledWith('tunnel-1');
	});

	it('should open the editor', async () => {
		setUp();
		renderDrawer();

		await userEvent.click(screen.getByRole('button', { name: /Modifier/ }));

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

			expect(
				await screen.findByText('Aucun événement enregistré')
			).toBeInTheDocument();
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
			expect(within(log).getByText('Avertissement')).toBeInTheDocument();
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
				new DaemonCommandError('not_found', 'tunnel introuvable')
			);
			renderDrawer(gateway);

			expect(await screen.findByRole('alert')).toHaveTextContent(
				'Journal indisponible : tunnel introuvable'
			);
			await userEvent.click(screen.getByRole('button', { name: 'Réessayer' }));

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
		expect(screen.getAllByText('Temporaire')).toHaveLength(1);
		expect(
			within(actions).queryByRole('button', { name: 'Modifier' })
		).not.toBeInTheDocument();
		expect(
			within(actions).getByRole('button', { name: 'Enregistrer le tunnel' })
		).toBeInTheDocument();
		expect(
			within(actions).getByRole('button', { name: 'Arrêter et retirer' })
		).toBeInTheDocument();
	});

	it('should save the tunnel when the save action is clicked', async () => {
		setUp({ isTemporary: true });
		const { gateway } = renderDrawer();

		await userEvent.click(
			screen.getByRole('button', { name: 'Enregistrer le tunnel' })
		);

		expect(gateway.persistTunnel).toHaveBeenCalledWith('tunnel-1');
	});

	it('should have no axe violations', async () => {
		setUp({ isTemporary: true });
		renderDrawer();

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
