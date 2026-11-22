import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { DevslotDiscoveryDto } from '#/daemon/protocol';
import { ImportDrawer } from '#/import/import_drawer';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { classifyAgainstStore } from '#/testing/classify_against_store';
import {
	buildDevslotDiscovery,
	buildDevslotInstance,
	buildDevslotPort,
	buildDevslotStale,
	buildSettings,
	buildTunnel,
} from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { findFilledLiveRegion } from '#/testing/filled_live_region';
import { renderWithGateway } from '#/testing/render_with_gateway';

type User = ReturnType<typeof userEvent.setup>;

const WEB_PORT = buildDevslotPort({
	var: 'PORT',
	port: 20_050,
	url: 'http://web-shop.localhost:20050',
	hostName: 'web-shop.localhost',
});
const SERVICE_PORT = buildDevslotPort({
	var: 'DB_PORT',
	port: 20_052,
	kind: 'service',
	service: 'postgres',
	target: 5432,
});
const PRIVILEGED_PORT = buildDevslotPort({ var: 'HTTP', port: 80 });
const TUNNELED_PORT = buildDevslotPort({
	var: 'CACHE_PORT',
	port: 20_053,
	status: 'alreadyTunneled',
	tunnelId: 'existing',
});

const buildFullDiscovery = (
	overrides: Partial<DevslotDiscoveryDto> = {}
): DevslotDiscoveryDto =>
	buildDevslotDiscovery({
		instances: [
			buildDevslotInstance({
				ports: [WEB_PORT, SERVICE_PORT, PRIVILEGED_PORT, TUNNELED_PORT],
			}),
		],
		...overrides,
	});

function renderDrawer(discovery: DevslotDiscoveryDto = buildFullDiscovery()) {
	const gateway = createFakeGateway();
	gateway.discoverDevslot.mockResolvedValue(discovery);
	gateway.classifyImports.mockImplementation(classifyAgainstStore);
	useDaemonStore.setState({
		settings: buildSettings(),
		hosts: ['nova', 'vega'],
	});
	useUiStore.getState().openImport();
	return renderWithGateway(<ImportDrawer />, gateway);
}

async function discover(user: User) {
	await user.click(screen.getByRole('tab', { name: 'From devslot' }));
	await user.selectOptions(
		screen.getByRole('combobox', { name: /Host/ }),
		'vega'
	);
	await user.click(screen.getByRole('button', { name: 'Discover' }));
	await screen.findByRole('heading', { name: 'Discovery result' });
}

const getTabStatus = () =>
	within(screen.getByRole('tabpanel')).getByRole('status');
const getPort = (name: RegExp) => screen.getByRole('checkbox', { name });

describe('Import from devslot', () => {
	beforeEach(() => {
		useAnnouncerStore.setState({
			polite: { id: 0, message: '' },
			assertive: { id: 0, message: '' },
		});
	});

	it('should ask for a host before discovering', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('tab', { name: 'From devslot' }));
		await user.click(screen.getByRole('button', { name: 'Discover' }));

		expect(screen.getByRole('combobox', { name: /Host/ })).toBeInvalid();
		expect(gateway.discoverDevslot).not.toHaveBeenCalled();
	});

	it('should discover the host and focus the result heading', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await discover(user);

		expect(gateway.discoverDevslot).toHaveBeenCalledWith('vega');
		expect(
			screen.getByRole('heading', { name: 'Discovery result' })
		).toHaveFocus();
		expect(
			screen.getByRole('group', { name: 'web-shop-feat-auth-3f2a' })
		).toBeInTheDocument();
	});

	it('should check the web ports only and disable the ports already tunneled with their reason', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await discover(user);

		expect(getPort(/^PORT/)).toBeChecked();
		expect(getPort(/^DB_PORT/)).not.toBeChecked();
		expect(getPort(/^HTTP/)).not.toBeChecked();
		expect(getPort(/^CACHE_PORT/)).toBeDisabled();
		expect(getPort(/^CACHE_PORT/)).toHaveAccessibleDescription(
			/a tunnel already forwards to this port/
		);
	});

	it('should show the app url as text, never as a link', async () => {
		const user = userEvent.setup();
		renderDrawer(
			buildFullDiscovery({
				instances: [
					buildDevslotInstance({
						appUrl: 'http://web-shop.localhost:20050',
						ports: [WEB_PORT],
					}),
				],
			})
		);

		await discover(user);

		expect(screen.queryByRole('link')).not.toBeInTheDocument();
		expect(
			screen.getAllByText(/http:\/\/web-shop\.localhost:20050/).length
		).toBeGreaterThan(0);
	});

	it('should have no accessibility violation on the result', async () => {
		const user = userEvent.setup();
		const { container } = renderDrawer(
			buildFullDiscovery({ stale: [buildDevslotStale()] })
		);

		await discover(user);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have a status region before any result', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('tab', { name: 'From devslot' }));

		expect(getTabStatus()).toBeInTheDocument();
	});

	it('should remove the stale tunnels marked remove without asking and announce them', async () => {
		const user = userEvent.setup();
		const stale = buildDevslotStale({
			tunnel: {
				id: 'gone',
				host: 'vega',
				kind: 'local',
				listenPort: 20_060,
				tags: [],
			},
			isTemporary: true,
			action: 'remove',
		});
		const { gateway } = renderDrawer(buildFullDiscovery({ stale: [stale] }));

		await discover(user);

		expect(gateway.resyncDevslot).toHaveBeenCalledWith('vega', [
			{ id: 'gone', definition: stale.tunnel, isTemporary: true },
		]);
		expect(
			await screen.findByText(/1 stale tunnel removed: 20060 → vega:20060/)
		).toBeInTheDocument();
		expect(getTabStatus()).toHaveTextContent('1 stale tunnel removed');
	});

	it('should not resync when nothing is marked remove', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer(
			buildFullDiscovery({ stale: [buildDevslotStale()] })
		);

		await discover(user);

		expect(gateway.resyncDevslot).not.toHaveBeenCalled();
	});

	it('should delete an ask tunnel with a resync of that tunnel only and move the focus to the next one', async () => {
		const user = userEvent.setup();
		const first = buildDevslotStale();
		const second = buildDevslotStale({
			tunnel: { ...first.tunnel, id: 'stale-2', listenPort: 20_061 },
		});
		const { gateway } = renderDrawer(
			buildFullDiscovery({ stale: [first, second] })
		);
		await discover(user);

		await user.click(
			screen.getByRole('button', { name: /^Delete 20060 to vega:20060/ })
		);

		expect(gateway.resyncDevslot).toHaveBeenCalledWith('vega', [
			{ id: 'stale-1', definition: first.tunnel, isTemporary: false },
		]);
		expect(
			await screen.findByRole('button', { name: /^Delete 20061/ })
		).toHaveFocus();
		expect(getTabStatus()).toHaveTextContent('Deleted 20060 to vega:20060');
	});

	it('should keep an ask tunnel without calling the daemon and focus the list heading when it was the last', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer(
			buildFullDiscovery({ stale: [buildDevslotStale()] })
		);
		await discover(user);

		await user.click(screen.getByRole('button', { name: /^Keep 20060/ }));

		expect(gateway.resyncDevslot).not.toHaveBeenCalled();
		expect(
			screen.getByRole('heading', { name: 'Saved tunnels to review' })
		).toHaveFocus();
		expect(
			screen.queryByRole('button', { name: /^Keep/ })
		).not.toBeInTheDocument();
	});

	it('should show the exact TOML to add when ds is not found', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.discoverDevslot.mockRejectedValue(
			new DaemonCommandError('devslot_not_found', 'raw', {
				host: 'vega',
				path: 'ds',
			})
		);

		await user.click(screen.getByRole('tab', { name: 'From devslot' }));
		await user.selectOptions(
			screen.getByRole('combobox', { name: /Host/ }),
			'vega'
		);
		await user.click(screen.getByRole('button', { name: 'Discover' }));

		const alert = await findFilledLiveRegion('alert');
		expect(alert).toHaveTextContent('devslot was not found on vega');
		expect(alert).toHaveFocus();
		expect(
			screen.getByText(/\[hosts\.vega\]\s*devslot_path = "\/path\/to\/ds"/)
		).toBeInTheDocument();
	});

	it('should start the checked ports as temporary tunnels with the devslot origin', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await discover(user);

		await user.click(screen.getByRole('button', { name: 'Start 1 tunnel' }));

		expect(gateway.startTemporaryTunnel).toHaveBeenCalledWith({
			host: 'vega',
			kind: 'local',
			listenPort: 20_050,
			listenAddress: '127.0.0.1',
			open: { hostName: 'web-shop.localhost' },
			devslot: { projectTag: 'ds-web-shop' },
		});
		expect(gateway.createTunnel).not.toHaveBeenCalled();
	});

	it('should create tagged tunnels then start them when the tunnels are kept', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await discover(user);

		await user.click(
			screen.getByRole('switch', { name: 'Keep these tunnels' })
		);
		await user.click(
			screen.getByRole('button', { name: 'Create and start 1 tunnel' })
		);

		expect(gateway.createTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ tags: ['devslot', 'ds-web-shop'] })
		);
		expect(gateway.startTunnel).toHaveBeenCalledWith('created-tunnel');
	});

	it('should start with the keep switch on when the daemon says so', async () => {
		const user = userEvent.setup();
		renderDrawer(buildFullDiscovery({ persistDefault: true }));

		await discover(user);

		expect(
			screen.getByRole('switch', { name: 'Keep these tunnels' })
		).toBeChecked();
	});

	it('should report a start failure apart from a creation success when tunnels are kept', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer(
			buildFullDiscovery({ persistDefault: true })
		);
		gateway.startTunnel.mockRejectedValue(
			new DaemonCommandError('connect_failed', 'no route')
		);
		await discover(user);

		await user.click(
			screen.getByRole('button', { name: 'Create and start 1 tunnel' })
		);

		const alert = await findFilledLiveRegion('alert');
		expect(alert).toHaveTextContent('0 started, 1 failed:');
		expect(alert).toHaveTextContent('was created but did not start: no route');
		expect(gateway.deleteTunnel).not.toHaveBeenCalled();
	});

	it('should let the user change the local port of a port that clashes with an existing tunnel', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({ host: 'vega', listenPort: 20_050, targetPort: 9 }),
			],
		});
		const { gateway } = renderDrawer();
		await discover(user);
		expect(getPort(/^PORT/)).not.toBeChecked();

		const field = screen.getByRole('textbox', {
			name: 'Local port for PORT of web-shop-feat-auth-3f2a',
		});
		await user.clear(field);
		await user.type(field, '30050');
		await user.click(getPort(/^PORT/));
		await user.click(screen.getByRole('button', { name: 'Start 1 tunnel' }));

		expect(field).toHaveAccessibleDescription(
			/the app builds its links with port 20050/
		);
		expect(gateway.startTemporaryTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ listenPort: 30_050, targetPort: 20_050 })
		);
	});

	it('should refuse to start while a chosen port clashes and focus its field', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({ host: 'vega', listenPort: 20_050, targetPort: 9 }),
			],
		});
		const { gateway } = renderDrawer();
		await discover(user);

		await user.click(getPort(/^PORT/));
		await user.click(screen.getByRole('button', { name: 'Start 1 tunnel' }));

		const field = screen.getByRole('textbox', {
			name: 'Local port for PORT of web-shop-feat-auth-3f2a',
		});
		expect(field).toHaveFocus();
		expect(field).toBeInvalid();
		expect(gateway.startTemporaryTunnel).not.toHaveBeenCalled();
	});

	it('should list the warnings of the discovery', async () => {
		const user = userEvent.setup();
		renderDrawer(
			buildFullDiscovery({
				warnings: [{ kind: 'invalidPort', instance: 'inst', var: 'X' }],
			})
		);

		await discover(user);

		const list = screen.getByRole('list');
		expect(
			within(list).getByText(/Port X of inst was skipped/)
		).toBeInTheDocument();
	});

	it('should make a port selectable again once its stale tunnel was removed', async () => {
		const user = userEvent.setup();
		const staleTunnel = {
			id: 'old-temporary',
			host: 'vega',
			kind: 'local' as const,
			listenPort: 20_050,
			tags: [],
		};
		renderDrawer(
			buildFullDiscovery({
				instances: [
					buildDevslotInstance({
						ports: [
							buildDevslotPort({
								status: 'alreadyTunneled',
								tunnelId: 'old-temporary',
							}),
						],
					}),
				],
				stale: [
					buildDevslotStale({
						tunnel: staleTunnel,
						isTemporary: true,
						action: 'remove',
					}),
				],
			})
		);

		await discover(user);

		await waitFor(() => expect(getPort(/^PORT/)).toBeEnabled());
		expect(getPort(/^PORT/)).toBeChecked();
	});

	it('should make a port selectable again once its ask tunnel was deleted', async () => {
		const user = userEvent.setup();
		renderDrawer(
			buildFullDiscovery({
				instances: [
					buildDevslotInstance({
						ports: [
							buildDevslotPort({
								status: 'alreadyTunneled',
								tunnelId: 'stale-1',
							}),
						],
					}),
				],
				stale: [buildDevslotStale()],
			})
		);
		await discover(user);
		expect(getPort(/^PORT/)).toBeDisabled();

		await user.click(screen.getByRole('button', { name: /^Delete 20060/ }));

		await waitFor(() => expect(getPort(/^PORT/)).toBeEnabled());
	});

	it('should report a failed delete under its own title and keep the tunnel listed', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer(
			buildFullDiscovery({ stale: [buildDevslotStale()] })
		);
		await discover(user);
		gateway.resyncDevslot.mockRejectedValue(
			new DaemonCommandError('internal', 'raw')
		);

		await user.click(screen.getByRole('button', { name: /^Delete 20060/ }));

		const alert = await findFilledLiveRegion('alert');
		expect(alert).toHaveTextContent('Could not delete 20060 to vega:20060:');
		expect(screen.getByRole('button', { name: /^Delete 20060/ })).toBeEnabled();
	});

	it('should ignore a second press on Delete while it runs', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer(
			buildFullDiscovery({ stale: [buildDevslotStale()] })
		);
		await discover(user);
		let release: () => void = () => undefined;
		gateway.resyncDevslot.mockReturnValue(
			new Promise((resolve) => {
				release = () => resolve({ removed: ['stale-1'], skipped: [] });
			})
		);
		const button = screen.getByRole('button', { name: /^Delete 20060/ });

		await user.click(button);
		await user.click(button);
		act(() => release());

		await waitFor(() =>
			expect(
				screen.queryByRole('button', { name: /^Delete/ })
			).not.toBeInTheDocument()
		);
		expect(gateway.resyncDevslot).toHaveBeenCalledTimes(1);
	});

	it('should focus the host select when no host is chosen', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('tab', { name: 'From devslot' }));
		await user.click(screen.getByRole('button', { name: 'Discover' }));

		expect(screen.getByRole('combobox', { name: /Host/ })).toHaveFocus();
	});

	it('should say that it is discovering while the daemon works', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		let release: () => void = () => undefined;
		gateway.discoverDevslot.mockReturnValue(
			new Promise((resolve) => {
				release = () => resolve(buildFullDiscovery());
			})
		);
		await user.click(screen.getByRole('tab', { name: 'From devslot' }));
		await user.selectOptions(
			screen.getByRole('combobox', { name: /Host/ }),
			'vega'
		);

		await user.click(screen.getByRole('button', { name: 'Discover' }));

		expect(getTabStatus()).toHaveTextContent('Discovering on vega…');
		act(() => release());
		await screen.findByRole('heading', { name: 'Discovery result' });
		expect(getTabStatus()).toBeEmptyDOMElement();
	});

	it('should describe the result heading with the instance count', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await discover(user);

		expect(
			screen.getByRole('heading', { name: 'Discovery result' })
		).toHaveAccessibleDescription('1 devslot instance found');
	});

	it('should describe an instance with its details', async () => {
		const user = userEvent.setup();
		renderDrawer(
			buildFullDiscovery({
				instances: [
					buildDevslotInstance({ servicesRunning: false, ports: [WEB_PORT] }),
				],
			})
		);

		await discover(user);

		expect(
			screen.getByRole('group', { name: 'web-shop-feat-auth-3f2a' })
		).toHaveAccessibleDescription(/Services are not running/);
	});

	it('should announce the start once and keep a status region while starting', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.startTemporaryTunnel.mockRejectedValue(
			new DaemonCommandError('port_in_use', 'busy')
		);
		await discover(user);

		await user.click(screen.getByRole('button', { name: 'Start 1 tunnel' }));

		await findFilledLiveRegion('alert');
		const outputs = screen.getAllByRole('status');
		expect(outputs.at(-1)).toHaveTextContent('0 started, 1 failed');
		expect(
			screen.getByRole('button', { name: /^Start 1 tunnel/ }).closest('form')
		).not.toHaveAttribute('aria-busy');
	});

	it('should offer to copy the configuration snippet when ds is not found', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.discoverDevslot.mockRejectedValue(
			new DaemonCommandError('devslot_not_found', 'raw', {
				host: 'vega',
				path: 'ds',
			})
		);
		await user.click(screen.getByRole('tab', { name: 'From devslot' }));
		await user.selectOptions(
			screen.getByRole('combobox', { name: /Host/ }),
			'vega'
		);

		await user.click(screen.getByRole('button', { name: 'Discover' }));

		expect(
			await screen.findByRole('button', { name: 'Copy configuration snippet' })
		).toBeInTheDocument();
	});

	it('should add the conflict to the description of a ticked port', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({ host: 'vega', listenPort: 20_050, targetPort: 9 }),
			],
		});
		renderDrawer();
		await discover(user);

		await user.click(getPort(/^PORT/));

		expect(getPort(/^PORT/)).toHaveAccessibleDescription(
			/Port already used by/
		);
	});

	it('should ask the daemon about the chosen ports first, then the others', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer(
			buildFullDiscovery({
				instances: [
					buildDevslotInstance({
						ports: [
							buildDevslotPort({ var: 'A', port: 20_050, kind: 'service' }),
							buildDevslotPort({ var: 'B', port: 20_051 }),
						],
					}),
				],
			})
		);
		await discover(user);

		await waitFor(() =>
			expect(gateway.classifyImports).toHaveBeenLastCalledWith([
				expect.objectContaining({ listenPort: 20_051 }),
				expect.objectContaining({ listenPort: 20_050 }),
			])
		);
		await user.click(getPort(/^A/));

		await waitFor(() =>
			expect(gateway.classifyImports).toHaveBeenLastCalledWith([
				expect.objectContaining({ listenPort: 20_050 }),
				expect.objectContaining({ listenPort: 20_051 }),
			])
		);
	});

	it('should show the conflict of a port that is not chosen', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({ host: 'vega', listenPort: 20_050, targetPort: 9 }),
			],
		});
		renderDrawer();
		await discover(user);

		expect(
			await screen.findByRole('textbox', {
				name: 'Local port for PORT of web-shop-feat-auth-3f2a',
			})
		).toHaveAccessibleDescription(/Port already used by/);
	});

	it('should keep the start button disabled while the daemon has not answered', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await discover(user);
		await screen.findByRole('button', { name: 'Start 1 tunnel' });
		let answer: (conflicts: null[]) => void = () => undefined;
		gateway.classifyImports.mockImplementation(
			() =>
				new Promise((resolve) => {
					answer = resolve;
				})
		);

		await user.click(getPort(/^HTTP/));

		const pendingButton = screen.getByRole('button', {
			name: 'Start 2 tunnels',
		});
		expect(pendingButton).toHaveAttribute('aria-disabled', 'true');
		expect(pendingButton).toHaveAccessibleDescription(
			'Checking for conflicts…'
		);
		await user.click(pendingButton);
		expect(gateway.startTemporaryTunnel).not.toHaveBeenCalled();
		expect(await axe(document.body)).toHaveNoViolations();
		act(() => answer([null, null, null]));
		await waitFor(() =>
			expect(
				screen.getByRole('button', { name: 'Start 2 tunnels' })
			).toBeEnabled()
		);
	});

	it('should keep the start button focusable and say why when no port is chosen', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await discover(user);
		await user.click(getPort(/^PORT/));
		const button = screen.getByRole('button', { name: 'Start 0 tunnels' });

		button.focus();
		await user.click(button);

		expect(button).toHaveAttribute('aria-disabled', 'true');
		expect(button).toHaveAccessibleDescription('Select at least one port');
		expect(button).toHaveFocus();
		expect(gateway.startTemporaryTunnel).not.toHaveBeenCalled();
	});

	it('should say so when the check for conflicts fails and keep the start possible', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.classifyImports.mockRejectedValue(
			new DaemonCommandError('disconnected', 'lost')
		);
		await discover(user);

		expect(
			await screen.findByText(
				'Could not check for conflicts; creation will still refuse clashes.'
			)
		).toBeInTheDocument();
		expect(
			screen.getByRole('button', { name: 'Start 1 tunnel' })
		).not.toHaveAttribute('aria-disabled');
	});

	describe('with a local port busy on this computer', () => {
		const fieldName = /^Local port for PORT of /;
		const renderBusy = (suggestedLocalPort?: number) =>
			renderDrawer(
				buildFullDiscovery({
					instances: [
						buildDevslotInstance({
							ports: [
								buildDevslotPort({ localPortBusy: true, suggestedLocalPort }),
							],
						}),
					],
				})
			);

		it('should prefill the suggested local port, keep the port checked and start with it', async () => {
			const user = userEvent.setup();
			const { gateway } = renderBusy(30_050);
			await discover(user);

			expect(getPort(/^PORT/)).toBeChecked();
			const field = screen.getByRole('textbox', { name: fieldName });
			expect(field).toHaveValue('30050');
			expect(field).toHaveAccessibleDescription(/with port 20050/);
			await user.click(screen.getByRole('button', { name: 'Start 1 tunnel' }));

			expect(gateway.startTemporaryTunnel).toHaveBeenCalledWith(
				expect.objectContaining({ listenPort: 30_050, targetPort: 20_050 })
			);
		});

		it('should not preselect a busy port without suggestion and explain it', async () => {
			const user = userEvent.setup();
			renderBusy();
			await discover(user);

			expect(getPort(/^PORT/)).not.toBeChecked();
			expect(
				screen.getByRole('textbox', { name: fieldName })
			).toHaveAccessibleDescription(
				'Port 20050 is already in use on this computer: choose another local port.'
			);
		});

		it('should block the start and focus the field while the busy port is unchanged', async () => {
			const user = userEvent.setup();
			const { gateway } = renderBusy();
			await discover(user);

			await user.click(getPort(/^PORT/));
			expect(getPort(/^PORT/)).toHaveAccessibleDescription(
				/already in use on this computer/
			);
			await user.click(screen.getByRole('button', { name: 'Start 1 tunnel' }));

			const field = screen.getByRole('textbox', { name: fieldName });
			expect(field).toHaveFocus();
			expect(field).toBeInvalid();
			expect(gateway.startTemporaryTunnel).not.toHaveBeenCalled();
		});

		it('should have no accessibility violation', async () => {
			const user = userEvent.setup();
			const { container } = renderBusy();
			await discover(user);

			expect(await axe(container)).toHaveNoViolations();
		});
	});

	it('should keep the local port field of a port whose start failed', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.startTemporaryTunnel.mockRejectedValue(
			new DaemonCommandError('port_in_use', 'busy')
		);
		await discover(user);
		const fieldName = /^Local port for PORT of /;
		expect(
			screen.queryByRole('textbox', { name: fieldName })
		).not.toBeInTheDocument();

		await user.click(screen.getByRole('button', { name: 'Start 1 tunnel' }));

		expect(
			await screen.findByRole('textbox', { name: fieldName })
		).toBeInTheDocument();
	});
});
