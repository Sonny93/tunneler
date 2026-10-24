import { i18n } from '@lingui/core';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import {
	COLON_TAG_MESSAGE,
	DIGITS_TAG_MESSAGE,
} from '#/shared/reserved_tag_problem';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { buildOpener, buildSettings, buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { TunnelFormDrawer } from '#/tunnel_form/tunnel_form_drawer';

function openEditor(tunnelId: string | null = null) {
	useUiStore.getState().openTunnelEditor(tunnelId);
	useDaemonStore.setState({
		settings: buildSettings({ defaultBindAddress: '127.0.0.1' }),
		hosts: ['nova', 'prod'],
	});
}

const renderDrawer = (gateway = createFakeGateway()) =>
	renderWithGateway(<TunnelFormDrawer />, gateway);

async function fillRequiredFields(user: ReturnType<typeof userEvent.setup>) {
	await user.type(screen.getByRole('combobox', { name: /^Host/ }), 'nova');
	await user.type(
		screen.getByRole('textbox', { name: /^Listen port/ }),
		'3333'
	);
}

describe('TunnelFormDrawer', () => {
	it('should render nothing while no editor is open', () => {
		renderDrawer();

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should be titled "New tunnel" when creating', () => {
		openEditor();
		renderDrawer();

		expect(
			screen.getByRole('dialog', { name: 'New tunnel' })
		).toBeInTheDocument();
	});

	describe('in temporary mode', () => {
		const openTemporaryEditor = () => {
			useUiStore.getState().openTemporaryTunnelEditor();
			useDaemonStore.setState({
				settings: buildSettings({ defaultBindAddress: '127.0.0.1' }),
				hosts: ['nova'],
			});
		};

		it('should be titled "New temporary tunnel"', () => {
			openTemporaryEditor();
			renderDrawer();

			expect(
				screen.getByRole('dialog', { name: 'New temporary tunnel' })
			).toBeInTheDocument();
		});

		it('should explain that the tunnel is not saved', () => {
			openTemporaryEditor();
			renderDrawer();

			expect(
				screen.getByText(
					'Not saved: it disappears when it is stopped or when Tunneler quits.'
				)
			).toBeInTheDocument();
		});

		it('should have no standby choice', () => {
			openTemporaryEditor();
			renderDrawer();

			expect(
				screen.queryByRole('radiogroup', { name: 'Standby' })
			).not.toBeInTheDocument();
		});

		it('should have no tags field', () => {
			openTemporaryEditor();
			renderDrawer();

			expect(
				screen.queryByRole('textbox', { name: /^Tags/ })
			).not.toBeInTheDocument();
		});

		it('should have no axe violations', async () => {
			openTemporaryEditor();
			renderDrawer();

			expect(await axe(document.body)).toHaveNoViolations();
		});

		it('should start the tunnel without tags and close the drawer', async () => {
			openTemporaryEditor();
			const user = userEvent.setup();
			const { gateway } = renderDrawer();
			await fillRequiredFields(user);

			await user.click(screen.getByRole('button', { name: 'Start' }));

			await waitFor(() =>
				expect(useUiStore.getState().panel.kind).toBe('none')
			);
			expect(gateway.startTemporaryTunnel).toHaveBeenCalledWith({
				host: 'nova',
				kind: 'local',
				listenPort: 3333,
				listenAddress: undefined,
				targetHost: undefined,
				targetPort: undefined,
			});
			expect(gateway.createTunnel).not.toHaveBeenCalled();
		});

		it('should toast that the temporary tunnel started', async () => {
			openTemporaryEditor();
			const user = userEvent.setup();
			renderDrawer();
			await fillRequiredFields(user);

			await user.click(screen.getByRole('button', { name: 'Start' }));

			await waitFor(() =>
				expect(useToastStore.getState().toasts).toEqual([
					expect.objectContaining({
						message: 'Temporary tunnel 3333 → nova:3333 started',
					}),
				])
			);
		});

		it('should stay open and show the error when the start fails', async () => {
			openTemporaryEditor();
			const user = userEvent.setup();
			const gateway = createFakeGateway();
			gateway.startTemporaryTunnel.mockRejectedValue(
				new DaemonCommandError('io_error', 'Invalid port')
			);
			renderDrawer(gateway);
			await fillRequiredFields(user);

			await user.click(screen.getByRole('button', { name: 'Start' }));

			expect(await screen.findByText(/Invalid port/)).toBeInTheDocument();
			expect(useUiStore.getState().panel.kind).toBe('temporaryTunnelEditor');
		});
	});

	it('should have no axe violations', async () => {
		openEditor();
		renderDrawer();

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should offer the known hosts as suggestions', () => {
		openEditor();
		renderDrawer();

		const list = screen
			.getByRole('combobox', { name: /^Host/ })
			.getAttribute('list');

		expect(
			[...(document.getElementById(list ?? '')?.children ?? [])].map((option) =>
				option.getAttribute('value')
			)
		).toEqual(['nova', 'prod']);
	});

	it('should create the tunnel without target port while it is linked', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);

		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith({
			host: 'nova',
			kind: 'local',
			listenPort: 3333,
			listenAddress: undefined,
			targetHost: undefined,
			targetPort: undefined,
			tags: [],
		});
	});

	it('should close and toast after creating', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await fillRequiredFields(user);

		await user.click(screen.getByRole('button', { name: 'Create' }));

		await waitFor(() => expect(useUiStore.getState().panel.kind).toBe('none'));
		expect(useToastStore.getState().toasts).toEqual([
			expect.objectContaining({
				tone: 'success',
				message: 'Tunnel 3333 → nova:3333 created',
			}),
		]);
	});

	it('should own the submit button so that Enter submits the form', () => {
		openEditor();
		renderDrawer();

		expect(screen.getByRole('button', { name: 'Create' })).toHaveAttribute(
			'form',
			document.querySelector('form')?.id
		);
	});

	it('should show the listen port in the linked target port and follow it', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.type(
			screen.getByRole('textbox', { name: /^Listen port/ }),
			'8080'
		);

		expect(screen.getByRole('textbox', { name: /^Target port/ })).toHaveValue(
			'8080'
		);
		expect(screen.getByText('Follows the listen port')).toBeInTheDocument();
	});

	it('should unlink the target port when it is edited and send it', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		const targetPort = screen.getByRole('textbox', { name: /^Target port/ });

		await user.clear(targetPort);
		await user.type(targetPort, '5432');
		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ listenPort: 3333, targetPort: 5432 })
		);
	});

	it('should relink the target port with the relink button', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await user.type(
			screen.getByRole('textbox', { name: /^Listen port/ }),
			'3333'
		);
		const targetPort = screen.getByRole('textbox', { name: /^Target port/ });
		await user.clear(targetPort);
		await user.type(targetPort, '22');

		await user.click(
			screen.getByRole('button', { name: 'Link to the listen port' })
		);

		expect(targetPort).toHaveValue('3333');
		expect(screen.getByText('Follows the listen port')).toBeInTheDocument();
	});

	it('should parse the tags', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);

		await user.type(
			screen.getByRole('textbox', { name: 'Tags, separated by commas' }),
			' db, prod ,db,,'
		);
		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ tags: ['db', 'prod'] })
		);
	});

	it('should invalidate empty required fields and focus the first one', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Create' }));

		const host = screen.getByRole('combobox', { name: /^Host/ });
		expect(host).toHaveFocus();
		expect(host).toBeInvalid();
		expect(host).toHaveAccessibleDescription(
			expect.stringContaining('The host is required.')
		);
		expect(gateway.createTunnel).not.toHaveBeenCalled();
	});

	it('should explain an invalid port', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await fillRequiredFields(user);
		const listenPort = screen.getByRole('textbox', { name: /^Listen port/ });
		await user.clear(listenPort);
		await user.type(listenPort, '70000');

		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(listenPort).toHaveAccessibleDescription(
			'The port must be a whole number between 1 and 65535.'
		);
	});

	it('should show the daemon validation messages in a focused alert', async () => {
		openEditor();
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.createTunnel.mockRejectedValueOnce(
			new DaemonCommandError('validation_failed', 'invalide', [
				'Port 3333 is already in use.',
				'Unknown host.',
			])
		);
		renderDrawer(gateway);
		await fillRequiredFields(user);

		await user.click(screen.getByRole('button', { name: 'Create' }));

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent('Port 3333 is already in use.');
		expect(alert).toHaveTextContent('Unknown host.');
		expect(alert).toHaveFocus();
		expect(useUiStore.getState().panel.kind).toBe('tunnelEditor');
	});

	it('should show other errors with their message', async () => {
		openEditor();
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.createTunnel.mockRejectedValueOnce(
			new DaemonCommandError('io_error', 'Disque plein')
		);
		renderDrawer(gateway);
		await fillRequiredFields(user);

		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(await screen.findByRole('alert')).toHaveTextContent('Disque plein');
	});

	it('should close without saving on cancel', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Cancel' }));

		expect(useUiStore.getState().panel.kind).toBe('none');
		expect(gateway.createTunnel).not.toHaveBeenCalled();
	});
});

describe('TunnelFormDrawer kind selector', () => {
	const kindRadio = (name: RegExp) => screen.getByRole('radio', { name });

	it('should offer the three kinds in a radio group named "Tunnel type"', () => {
		openEditor();
		renderDrawer();

		expect(
			within(screen.getByRole('radiogroup', { name: 'Tunnel type' }))
				.getAllByRole('radio')
				.map((radio) => radio.getAttribute('value'))
		).toEqual(['local', 'remote', 'dynamic']);
	});

	it('should select local by default', () => {
		openEditor();
		renderDrawer();

		expect(kindRadio(/Local/)).toBeChecked();
	});

	it('should describe the direction of each kind', () => {
		openEditor();
		renderDrawer();

		expect(
			[/Local/, /Remote/, /Dynamic/].map((name) =>
				kindRadio(name).getAttribute('aria-describedby')
			)
		).not.toContain(null);
		expect(kindRadio(/Remote/)).toHaveAccessibleDescription(
			'Listens on the server, target reached from this computer'
		);
	});

	it('should move the selection with the arrow keys', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		kindRadio(/Local/).focus();

		await user.keyboard('{ArrowDown}');

		expect(kindRadio(/Remote/)).toBeChecked();
	});

	it('should say where things are for a remote tunnel', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(kindRadio(/Remote/));

		expect(
			screen.getByRole('textbox', { name: /^Listen port on the server/ })
		).toBeInTheDocument();
		expect(
			screen.getByRole('textbox', {
				name: /^Target host \(from this computer\)/,
			})
		).toBeInTheDocument();
	});

	it('should show the remote default bind address for a remote tunnel', async () => {
		openEditor();
		useDaemonStore.setState({
			settings: buildSettings({
				defaultBindAddress: '127.0.0.1',
				defaultRemoteBindAddress: '0.0.0.0',
			}),
		});
		const user = userEvent.setup();
		renderDrawer();

		await user.click(kindRadio(/Remote/));

		expect(
			screen.getByRole('textbox', { name: /^Listen address/ })
		).toHaveAccessibleDescription(/0\.0\.0\.0/);
	});

	it('should hide the target fields, keep focus on the radio and have no axe violations when the kind becomes dynamic', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(kindRadio(/Dynamic/));

		expect(
			screen.queryByRole('textbox', { name: /^Target port/ })
		).not.toBeInTheDocument();
		expect(
			screen.queryByRole('textbox', { name: /^Target host/ })
		).not.toBeInTheDocument();
		expect(kindRadio(/Dynamic/)).toHaveFocus();
		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should create a dynamic tunnel without target', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		await user.type(
			screen.getByRole('textbox', { name: /^Target host/ }),
			'db'
		);
		await user.click(kindRadio(/Dynamic/));

		await user.click(screen.getByRole('button', { name: 'Create' }));

		await waitFor(() =>
			expect(gateway.createTunnel).toHaveBeenCalledWith(
				expect.objectContaining({
					kind: 'dynamic',
					targetHost: undefined,
					targetPort: undefined,
				})
			)
		);
	});

	it('should restore the typed target when switching back from dynamic', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await user.type(
			screen.getByRole('textbox', { name: /^Target host/ }),
			'db'
		);
		await user.click(kindRadio(/Dynamic/));

		await user.click(kindRadio(/Local/));

		expect(screen.getByRole('textbox', { name: /^Target host/ })).toHaveValue(
			'db'
		);
	});

	it('should create a remote tunnel with its target', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		await user.type(
			screen.getByRole('textbox', { name: /^Target host/ }),
			'db'
		);
		await user.click(kindRadio(/Remote/));

		await user.click(screen.getByRole('button', { name: 'Create' }));

		await waitFor(() =>
			expect(gateway.createTunnel).toHaveBeenCalledWith(
				expect.objectContaining({ kind: 'remote', targetHost: 'db' })
			)
		);
	});
});

describe('TunnelFormDrawer opening options', () => {
	it('should offer the opening fieldset for a local tunnel', () => {
		openEditor();
		renderDrawer();

		expect(screen.getByRole('group', { name: 'Opening' })).toBeInTheDocument();
	});

	it('should hide the opening fieldset and the standby choice for a remote tunnel', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('radio', { name: /Remote/ }));

		expect(
			screen.queryByRole('group', { name: 'Opening' })
		).not.toBeInTheDocument();
		expect(
			screen.queryByRole('radiogroup', { name: 'Standby' })
		).not.toBeInTheDocument();
	});

	it('should mention the current general host in the inherited choice', () => {
		openEditor();
		renderDrawer();

		expect(
			screen.getByRole('radio', { name: 'General setting (localhost)' })
		).toBeChecked();
	});

	it('should offer the standby choice inheriting the general setting', () => {
		openEditor();
		renderDrawer();

		expect(
			screen.getByRole('radio', { name: 'General setting (no)' })
		).toBeChecked();
	});

	it('should describe the standby choice with its start-up hint', () => {
		openEditor();
		renderDrawer();

		expect(
			screen.getByRole('radiogroup', { name: 'Standby' })
		).toHaveAccessibleDescription(/Takes effect on the next start/);
	});

	it('should keep the standby choice for a dynamic tunnel', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('radio', { name: /Dynamic/ }));

		expect(
			screen.getByRole('radiogroup', { name: 'Standby' })
		).toBeInTheDocument();
	});

	it('should send the chosen standby when creating', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		await user.click(screen.getByRole('radio', { name: 'Yes' }));

		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ standby: true })
		);
	});

	it('should preview the final url', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await fillRequiredFields(user);

		await user.click(screen.getByRole('radio', { name: /^https/ }));
		await user.type(screen.getByRole('textbox', { name: /^Path/ }), '/admin');

		expect(
			screen.getByText('Opens https://localhost:3333/admin')
		).toBeInTheDocument();
	});

	it('should send the open options when creating', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		await user.click(screen.getByRole('radio', { name: /^https/ }));
		await user.type(screen.getByRole('textbox', { name: /^Path/ }), '/admin');
		await user.click(screen.getByRole('radio', { name: /^IP address/ }));

		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith(
			expect.objectContaining({
				open: { scheme: 'https', path: '/admin', host: 'ip' },
			})
		);
	});

	it('should flag an invalid path and not submit', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		await user.type(screen.getByRole('textbox', { name: /^Path/ }), 'admin');

		await user.click(screen.getByRole('button', { name: 'Create' }));

		const path = screen.getByRole('textbox', { name: /^Path/ });
		expect(path).toBeInvalid();
		expect(path).toHaveAccessibleDescription(
			expect.stringContaining('The path must start with "/".')
		);
		expect(gateway.createTunnel).not.toHaveBeenCalled();
	});

	it('should have no axe violations', async () => {
		openEditor();
		renderDrawer();

		expect(await axe(document.body)).toHaveNoViolations();
	});
});

describe('TunnelFormDrawer application choice', () => {
	const vnc = buildOpener({ ports: [3333] });
	const applicationSelect = () =>
		screen.getByRole('combobox', { name: 'Application' });

	it('should name the suggested application in the automatic choice', async () => {
		openEditor();
		useDaemonStore.setState({ openers: [vnc] });
		const user = userEvent.setup();
		renderDrawer();

		await fillRequiredFields(user);

		expect(
			within(applicationSelect()).getByRole('option', {
				name: 'Automatic (Remmina (VNC))',
			})
		).toBeInTheDocument();
	});

	it('should name the browser in the automatic choice when the general setting opens the browser', async () => {
		openEditor();
		useDaemonStore.setState({
			openers: [vnc],
			settings: buildSettings({ openDefault: 'browser' }),
		});
		const user = userEvent.setup();
		renderDrawer();

		await fillRequiredFields(user);

		expect(
			within(applicationSelect()).getByRole('option', {
				name: 'Automatic (Browser)',
			})
		).toBeInTheDocument();
	});

	it('should name the browser in the automatic choice when nothing is suggested', () => {
		openEditor();
		renderDrawer();

		expect(
			within(applicationSelect()).getByRole('option', {
				name: 'Automatic (Browser)',
			})
		).toBeInTheDocument();
	});

	it('should list the browser and every available application without axe violations', async () => {
		openEditor();
		useDaemonStore.setState({ openers: [vnc] });
		renderDrawer();

		expect(
			within(applicationSelect())
				.getAllByRole('option')
				.map((option) => option.textContent)
		).toEqual(['Automatic (Browser)', 'Browser', 'Remmina (VNC)']);
		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should send the chosen application', async () => {
		openEditor();
		useDaemonStore.setState({ openers: [vnc] });
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		await user.selectOptions(applicationSelect(), 'remmina-vnc');

		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ open: { with: 'remmina-vnc' } })
		);
	});

	it('should preview the command line of the chosen application', async () => {
		openEditor();
		useDaemonStore.setState({ openers: [vnc] });
		const user = userEvent.setup();
		renderDrawer();
		await fillRequiredFields(user);

		await user.selectOptions(applicationSelect(), 'remmina-vnc');

		expect(
			screen.getByText('Runs remmina -c vnc://localhost:3333')
		).toBeInTheDocument();
	});

	it('should keep an application that is not available anymore, flagged', () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ open: { with: 'gone' } })],
		});
		openEditor('tunnel-1');
		renderDrawer();

		expect(
			within(applicationSelect()).getByRole('option', {
				name: 'gone (unavailable)',
			})
		).toBeInTheDocument();
	});
});

describe('TunnelFormDrawer when editing', () => {
	const storeTunnel = (overrides = {}) => {
		const tunnel = buildTunnel({
			id: 'tunnel-7',
			listenPort: 5433,
			kind: 'remote',
			tags: ['db'],
			targetHost: 'db.internal',
			...overrides,
		});
		useDaemonStore.setState({ tunnels: [tunnel] });
		openEditor('tunnel-7');
	};

	it('should be titled with the tunnel endpoint', () => {
		storeTunnel();
		renderDrawer();

		expect(
			screen.getByRole('dialog', {
				name: 'Edit nova:5433 → db.internal:5433',
			})
		).toBeInTheDocument();
	});

	it('should prefill the fields', () => {
		storeTunnel();
		renderDrawer();

		expect([
			screen.getByRole('combobox', { name: /^Host/ }),
			screen.getByRole('textbox', { name: /^Listen port/ }),
			screen.getByRole('textbox', { name: /^Target host/ }),
			screen.getByRole('textbox', { name: /^Tags/ }),
		]).toEqual([
			expect.objectContaining({ value: 'nova' }),
			expect.objectContaining({ value: '5433' }),
			expect.objectContaining({ value: 'db.internal' }),
			expect.objectContaining({ value: 'db' }),
		]);
	});

	it('should link the target port when the tunnel has none', () => {
		storeTunnel();
		renderDrawer();

		expect(screen.getByText('Follows the listen port')).toBeInTheDocument();
	});

	it('should not link the target port when the tunnel has one', () => {
		storeTunnel({ targetPort: 5432 });
		renderDrawer();

		expect(screen.getByRole('textbox', { name: /^Target port/ })).toHaveValue(
			'5432'
		);
		expect(
			screen.getByRole('button', { name: 'Link to the listen port' })
		).toBeInTheDocument();
	});

	it('should select the kind of the tunnel', () => {
		storeTunnel();
		renderDrawer();

		expect(screen.getByRole('radio', { name: /Remote/ })).toBeChecked();
	});

	it('should update the tunnel keeping its kind', async () => {
		storeTunnel();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(gateway.updateTunnel).toHaveBeenCalledWith(
			expect.objectContaining({
				id: 'tunnel-7',
				kind: 'remote',
				targetPort: undefined,
			})
		);
	});

	it('should toast "saved" after updating', async () => {
		storeTunnel();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]?.message).toBe(
				'Tunnel nova:5433 → db.internal:5433 saved'
			)
		);
	});

	it('should close when the edited tunnel no longer exists', () => {
		useDaemonStore.setState({ tunnels: [] });
		openEditor('gone');
		renderDrawer();

		expect(useUiStore.getState().panel.kind).toBe('none');
	});
});

describe('TunnelFormDrawer field errors', () => {
	it('should show the host error when submitting without a host', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Create' }));

		expect(screen.getByText('The host is required.')).toBeInTheDocument();
	});

	it('should clear the host error when the user types a host', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await user.click(screen.getByRole('button', { name: 'Create' }));

		await user.type(screen.getByRole('combobox', { name: /^Host/ }), 'b');

		expect(screen.queryByText('The host is required.')).not.toBeInTheDocument();
	});

	it.each([
		['3333', DIGITS_TAG_MESSAGE],
		['nova:3333', COLON_TAG_MESSAGE],
	])(
		'should reject the tag "%s" the CLI would read as a port selector',
		async (tag, message) => {
			openEditor();
			const user = userEvent.setup();
			const { gateway } = renderDrawer();
			await fillRequiredFields(user);
			const tags = screen.getByRole('textbox', {
				name: 'Tags, separated by commas',
			});

			await user.type(tags, tag);
			await user.click(screen.getByRole('button', { name: 'Create' }));

			expect(tags).toHaveAccessibleDescription(i18n._(message));
			expect(gateway.createTunnel).not.toHaveBeenCalled();
		}
	);
});
