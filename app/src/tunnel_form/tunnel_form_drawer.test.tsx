import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { screen, waitFor, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
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
	await user.type(screen.getByRole('textbox', { name: /^Nom/ }), 'Adonis');
	await user.type(screen.getByRole('combobox', { name: /^Host/ }), 'nova');
	await user.type(
		screen.getByRole('textbox', { name: /^Port d'écoute/ }),
		'3333'
	);
}

describe('TunnelFormDrawer', () => {
	it('should render nothing while no editor is open', () => {
		renderDrawer();

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should be titled "Nouveau tunnel" when creating', () => {
		openEditor();
		renderDrawer();

		expect(
			screen.getByRole('dialog', { name: 'Nouveau tunnel' })
		).toBeInTheDocument();
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

		await user.click(screen.getByRole('button', { name: 'Créer' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith({
			name: 'Adonis',
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

		await user.click(screen.getByRole('button', { name: 'Créer' }));

		await waitFor(() => expect(useUiStore.getState().panel.kind).toBe('none'));
		expect(useToastStore.getState().toasts).toEqual([
			expect.objectContaining({
				tone: 'success',
				message: 'Tunnel Adonis créé',
			}),
		]);
	});

	it('should own the submit button so that Enter submits the form', () => {
		openEditor();
		renderDrawer();

		expect(screen.getByRole('button', { name: 'Créer' })).toHaveAttribute(
			'form',
			document.querySelector('form')?.id
		);
	});

	it('should show the listen port in the linked target port and follow it', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.type(
			screen.getByRole('textbox', { name: /^Port d'écoute/ }),
			'8080'
		);

		expect(screen.getByRole('textbox', { name: /^Port cible/ })).toHaveValue(
			'8080'
		);
		expect(screen.getByText("Suit le port d'écoute")).toBeInTheDocument();
	});

	it('should unlink the target port when it is edited and send it', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		const targetPort = screen.getByRole('textbox', { name: /^Port cible/ });

		await user.clear(targetPort);
		await user.type(targetPort, '5432');
		await user.click(screen.getByRole('button', { name: 'Créer' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ listenPort: 3333, targetPort: 5432 })
		);
	});

	it('should relink the target port with the relink button', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await user.type(
			screen.getByRole('textbox', { name: /^Port d'écoute/ }),
			'3333'
		);
		const targetPort = screen.getByRole('textbox', { name: /^Port cible/ });
		await user.clear(targetPort);
		await user.type(targetPort, '22');

		await user.click(
			screen.getByRole('button', { name: "Lier au port d'écoute" })
		);

		expect(targetPort).toHaveValue('3333');
		expect(screen.getByText("Suit le port d'écoute")).toBeInTheDocument();
	});

	it('should parse the tags', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);

		await user.type(
			screen.getByRole('textbox', { name: 'Tags séparés par des virgules' }),
			' db, prod ,db,,'
		);
		await user.click(screen.getByRole('button', { name: 'Créer' }));

		expect(gateway.createTunnel).toHaveBeenCalledWith(
			expect.objectContaining({ tags: ['db', 'prod'] })
		);
	});

	it('should invalidate empty required fields and focus the first one', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Créer' }));

		const name = screen.getByRole('textbox', { name: /^Nom/ });
		expect(name).toHaveFocus();
		expect(name).toBeInvalid();
		expect(name).toHaveAccessibleDescription('Le nom est obligatoire.');
		expect(gateway.createTunnel).not.toHaveBeenCalled();
	});

	it('should explain an invalid port', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await fillRequiredFields(user);
		const listenPort = screen.getByRole('textbox', { name: /^Port d'écoute/ });
		await user.clear(listenPort);
		await user.type(listenPort, '70000');

		await user.click(screen.getByRole('button', { name: 'Créer' }));

		expect(listenPort).toHaveAccessibleDescription(
			'Le port doit être un entier entre 1 et 65535.'
		);
	});

	it('should show the daemon validation messages in a focused alert', async () => {
		openEditor();
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.createTunnel.mockRejectedValueOnce(
			new DaemonCommandError('validation_failed', 'invalide', [
				'Le port 3333 est déjà utilisé.',
				'Nom en double.',
			])
		);
		renderDrawer(gateway);
		await fillRequiredFields(user);

		await user.click(screen.getByRole('button', { name: 'Créer' }));

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent('Le port 3333 est déjà utilisé.');
		expect(alert).toHaveTextContent('Nom en double.');
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

		await user.click(screen.getByRole('button', { name: 'Créer' }));

		expect(await screen.findByRole('alert')).toHaveTextContent('Disque plein');
	});

	it('should close without saving on cancel', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Annuler' }));

		expect(useUiStore.getState().panel.kind).toBe('none');
		expect(gateway.createTunnel).not.toHaveBeenCalled();
	});
});

describe('TunnelFormDrawer kind selector', () => {
	const kindRadio = (name: RegExp) => screen.getByRole('radio', { name });

	it('should offer the three kinds in a group named "Type de tunnel"', () => {
		openEditor();
		renderDrawer();

		expect(
			within(screen.getByRole('group', { name: 'Type de tunnel' }))
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
			[/Local/, /Distant/, /Dynamique/].map((name) =>
				kindRadio(name).getAttribute('aria-describedby')
			)
		).not.toContain(null);
		expect(kindRadio(/Distant/)).toHaveAccessibleDescription(
			'Écoute sur le serveur, cible vue depuis ce poste'
		);
	});

	it('should move the selection with the arrow keys', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		kindRadio(/Local/).focus();

		await user.keyboard('{ArrowDown}');

		expect(kindRadio(/Distant/)).toBeChecked();
	});

	it('should say where things are for a remote tunnel', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(kindRadio(/Distant/));

		expect(
			screen.getByRole('textbox', { name: /^Port d'écoute sur le serveur/ })
		).toBeInTheDocument();
		expect(
			screen.getByRole('textbox', { name: /^Hôte cible \(depuis ce poste\)/ })
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

		await user.click(kindRadio(/Distant/));

		expect(
			screen.getByRole('textbox', { name: /^Adresse d'écoute/ })
		).toHaveAccessibleDescription(/0\.0\.0\.0/);
	});

	it('should hide the target fields for a dynamic tunnel', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(kindRadio(/Dynamique/));

		expect(
			screen.queryByRole('textbox', { name: /^Port cible/ })
		).not.toBeInTheDocument();
		expect(
			screen.queryByRole('textbox', { name: /^Hôte cible/ })
		).not.toBeInTheDocument();
	});

	it('should keep focus on the radio when the kind changes', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(kindRadio(/Dynamique/));

		expect(kindRadio(/Dynamique/)).toHaveFocus();
	});

	it('should create a dynamic tunnel without target', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		await user.type(screen.getByRole('textbox', { name: /^Hôte cible/ }), 'db');
		await user.click(kindRadio(/Dynamique/));

		await user.click(screen.getByRole('button', { name: 'Créer' }));

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
		await user.type(screen.getByRole('textbox', { name: /^Hôte cible/ }), 'db');
		await user.click(kindRadio(/Dynamique/));

		await user.click(kindRadio(/Local/));

		expect(screen.getByRole('textbox', { name: /^Hôte cible/ })).toHaveValue(
			'db'
		);
	});

	it('should create a remote tunnel with its target', async () => {
		openEditor();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await fillRequiredFields(user);
		await user.type(screen.getByRole('textbox', { name: /^Hôte cible/ }), 'db');
		await user.click(kindRadio(/Distant/));

		await user.click(screen.getByRole('button', { name: 'Créer' }));

		await waitFor(() =>
			expect(gateway.createTunnel).toHaveBeenCalledWith(
				expect.objectContaining({ kind: 'remote', targetHost: 'db' })
			)
		);
	});

	it('should have no axe violations for a dynamic tunnel', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await user.click(kindRadio(/Dynamique/));

		expect(await axe(document.body)).toHaveNoViolations();
	});
});

describe('TunnelFormDrawer when editing', () => {
	const storeTunnel = (overrides = {}) => {
		const tunnel = buildTunnel({
			id: 'tunnel-7',
			name: 'Postgres',
			listenPort: 5433,
			kind: 'remote',
			tags: ['db'],
			targetHost: 'db.internal',
			...overrides,
		});
		useDaemonStore.setState({ tunnels: [tunnel] });
		openEditor('tunnel-7');
	};

	it('should be titled with the tunnel name', () => {
		storeTunnel();
		renderDrawer();

		expect(
			screen.getByRole('dialog', { name: 'Modifier Postgres' })
		).toBeInTheDocument();
	});

	it('should prefill the fields', () => {
		storeTunnel();
		renderDrawer();

		expect([
			screen.getByRole('textbox', { name: /^Nom/ }),
			screen.getByRole('textbox', { name: /^Port d'écoute/ }),
			screen.getByRole('textbox', { name: /^Hôte cible/ }),
			screen.getByRole('textbox', { name: /^Tags/ }),
		]).toEqual([
			expect.objectContaining({ value: 'Postgres' }),
			expect.objectContaining({ value: '5433' }),
			expect.objectContaining({ value: 'db.internal' }),
			expect.objectContaining({ value: 'db' }),
		]);
	});

	it('should link the target port when the tunnel has none', () => {
		storeTunnel();
		renderDrawer();

		expect(screen.getByText("Suit le port d'écoute")).toBeInTheDocument();
	});

	it('should not link the target port when the tunnel has one', () => {
		storeTunnel({ targetPort: 5432 });
		renderDrawer();

		expect(screen.getByRole('textbox', { name: /^Port cible/ })).toHaveValue(
			'5432'
		);
		expect(
			screen.getByRole('button', { name: "Lier au port d'écoute" })
		).toBeInTheDocument();
	});

	it('should select the kind of the tunnel', () => {
		storeTunnel();
		renderDrawer();

		expect(screen.getByRole('radio', { name: /Distant/ })).toBeChecked();
	});

	it('should update the tunnel keeping its kind', async () => {
		storeTunnel();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateTunnel).toHaveBeenCalledWith(
			expect.objectContaining({
				id: 'tunnel-7',
				kind: 'remote',
				name: 'Postgres',
				targetPort: undefined,
			})
		);
	});

	it('should toast "enregistré" after updating', async () => {
		storeTunnel();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]?.message).toBe(
				'Tunnel Postgres enregistré'
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
	it('should show the name error when submitting without a name', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Créer' }));

		expect(screen.getByText('Le nom est obligatoire.')).toBeInTheDocument();
	});

	it('should clear the name error when the user types a name', async () => {
		openEditor();
		const user = userEvent.setup();
		renderDrawer();
		await user.click(screen.getByRole('button', { name: 'Créer' }));

		await user.type(screen.getByRole('textbox', { name: /^Nom/ }), 'A');

		expect(
			screen.queryByText('Le nom est obligatoire.')
		).not.toBeInTheDocument();
	});
});
