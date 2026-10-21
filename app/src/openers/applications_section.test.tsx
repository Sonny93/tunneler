import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { screen, waitFor, within } from '@testing-library/react';

import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { buildOpener, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { ApplicationsSection } from '#/openers/applications_section';

const preset = buildOpener();
const mine = buildOpener({
	id: 'dbeaver',
	name: 'DBeaver',
	command: 'dbeaver',
	args: ['-con', 'host={host}'],
	ports: [5432],
	source: 'user',
});
const MINE_SPEC = {
	id: 'dbeaver',
	name: 'DBeaver',
	command: 'dbeaver',
	args: ['-con', 'host={host}'],
	ports: [5432],
};

const renderSection = (gateway = createFakeGateway()) =>
	renderWithGateway(
		<>
			<ApplicationsSection />
			<ModalProvider />
		</>,
		gateway
	);

describe('ApplicationsSection', () => {
	it('should list the detected applications with their command line, read only', () => {
		useDaemonStore.setState({ openers: [preset] });
		renderSection();

		const list = screen.getByRole('list', { name: 'Détectées sur ce poste' });
		expect(within(list).getByText('Remmina (VNC)')).toBeInTheDocument();
		expect(
			within(list).getByText('remmina -c vnc://{host}:{port}')
		).toBeInTheDocument();
		expect(
			within(list).queryByRole('button', { name: /Modifier|Supprimer/ })
		).not.toBeInTheDocument();
	});

	it('should say when nothing is detected or added', () => {
		renderSection();

		expect(
			screen.getByText('Aucune application détectée.')
		).toBeInTheDocument();
		expect(screen.getByText('Aucune application ajoutée.')).toBeInTheDocument();
	});

	it('should list the applications of the user with edit and delete buttons', () => {
		useDaemonStore.setState({ openers: [mine] });
		renderSection();

		expect(
			[
				screen.getByRole('button', { name: 'Modifier DBeaver' }),
				screen.getByRole('button', { name: 'Supprimer DBeaver' }),
			].length
		).toBe(2);
	});

	it('should propose an identifier from the name while adding', async () => {
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);

		await user.type(
			screen.getByRole('textbox', { name: /^Nom/ }),
			'Mon Client'
		);

		expect(screen.getByRole('textbox', { name: /^Identifiant/ })).toHaveValue(
			'mon-client'
		);
	});

	it('should stop following the name once the identifier is edited', async () => {
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Identifiant/ }), 'x');

		await user.type(screen.getByRole('textbox', { name: /^Nom/ }), 'Autre');

		expect(screen.getByRole('textbox', { name: /^Identifiant/ })).toHaveValue(
			'x'
		);
	});

	it('should state that arguments are not interpreted by a shell', async () => {
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);

		expect(
			screen.getByRole('textbox', { name: 'Arguments' })
		).toHaveAccessibleDescription(expect.stringContaining('aucun shell'));
	});

	it('should save the new application after the existing ones', async () => {
		useDaemonStore.setState({ openers: [mine] });
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Nom/ }), 'Viewer');
		await user.type(
			screen.getByRole('textbox', { name: /^Commande/ }),
			'viewer'
		);
		await user.type(
			screen.getByRole('textbox', { name: 'Arguments' }),
			'--host={{host}{enter}--port={{port}'
		);
		await user.type(
			screen.getByRole('textbox', { name: /^Ports/ }),
			'5900, 5901'
		);

		await user.click(screen.getByRole('button', { name: 'Ajouter' }));

		expect(gateway.updateOpeners).toHaveBeenCalledWith([
			MINE_SPEC,
			{
				id: 'viewer',
				name: 'Viewer',
				command: 'viewer',
				args: ['--host={host}', '--port={port}'],
				ports: [5900, 5901],
			},
		]);
	});

	it('should toast and close the form once saved', async () => {
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Nom/ }), 'Viewer');
		await user.type(
			screen.getByRole('textbox', { name: /^Commande/ }),
			'viewer'
		);

		await user.click(screen.getByRole('button', { name: 'Ajouter' }));

		await waitFor(() =>
			expect(
				useToastStore.getState().toasts.map((toast) => toast.message)
			).toEqual(['Application Viewer ajoutée'])
		);
		expect(
			screen.getByRole('button', { name: 'Ajouter une application' })
		).toBeInTheDocument();
	});

	it('should flag the invalid fields and not save', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);

		await user.click(screen.getByRole('button', { name: 'Ajouter' }));

		expect(screen.getByRole('textbox', { name: /^Nom/ })).toBeInvalid();
		expect(screen.getByRole('textbox', { name: /^Commande/ })).toBeInvalid();
		expect(gateway.updateOpeners).not.toHaveBeenCalled();
	});

	it('should refuse an identifier already used by another application', async () => {
		useDaemonStore.setState({ openers: [mine] });
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Nom/ }), 'DBeaver');
		await user.type(screen.getByRole('textbox', { name: /^Commande/ }), 'x');

		await user.click(screen.getByRole('button', { name: 'Ajouter' }));

		expect(
			screen.getByText(
				'Cet identifiant est déjà utilisé par une de vos applications.'
			)
		).toBeInTheDocument();
	});

	it('should keep the identifier read only when editing', async () => {
		useDaemonStore.setState({ openers: [mine] });
		const user = userEvent.setup();
		renderSection();

		await user.click(screen.getByRole('button', { name: 'Modifier DBeaver' }));

		expect(
			screen.getByRole('textbox', { name: /^Identifiant/ })
		).toHaveAttribute('readonly');
	});

	it('should replace the edited application and keep the others', async () => {
		useDaemonStore.setState({ openers: [mine] });
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(screen.getByRole('button', { name: 'Modifier DBeaver' }));
		const command = screen.getByRole('textbox', { name: /^Commande/ });
		await user.clear(command);
		await user.type(command, '/opt/dbeaver');

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateOpeners).toHaveBeenCalledWith([
			{ ...MINE_SPEC, command: '/opt/dbeaver' },
		]);
	});

	it('should show why the daemon refused the application', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.updateOpeners.mockRejectedValue(
			new DaemonCommandError('validation_failed', 'invalide', ['commande vide'])
		);
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Nom/ }), 'Viewer');
		await user.type(
			screen.getByRole('textbox', { name: /^Commande/ }),
			'viewer'
		);

		await user.click(screen.getByRole('button', { name: 'Ajouter' }));

		expect(await screen.findByRole('alert')).toHaveTextContent('commande vide');
	});

	it('should cancel without saving', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);

		await user.click(screen.getByRole('button', { name: 'Annuler' }));

		expect(gateway.updateOpeners).not.toHaveBeenCalled();
		expect(
			screen.queryByRole('form', { name: 'Ajouter une application' })
		).not.toBeInTheDocument();
	});

	it('should delete an unused application at once', async () => {
		useDaemonStore.setState({
			openers: [mine, buildOpener({ id: 'b', name: 'B', source: 'user' })],
		});
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);

		await user.click(screen.getByRole('button', { name: 'Supprimer DBeaver' }));

		await waitFor(() =>
			expect(gateway.updateOpeners).toHaveBeenCalledWith([
				expect.objectContaining({ id: 'b' }),
			])
		);
	});

	it('should ask for a confirmation stating how many tunnels use the application', async () => {
		useDaemonStore.setState({
			openers: [mine],
			tunnels: [
				buildTunnel({ id: 't1', open: { with: 'dbeaver' } }),
				buildTunnel({ id: 't2', open: { with: 'dbeaver' } }),
			],
		});
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);

		await user.click(screen.getByRole('button', { name: 'Supprimer DBeaver' }));

		const dialog = await screen.findByRole('alertdialog', {
			name: /Supprimer l’application DBeaver/,
		});
		expect(dialog).toHaveTextContent('2 tunnels utilisent cette application');
		expect(gateway.updateOpeners).not.toHaveBeenCalled();
	});

	it('should delete after the confirmation', async () => {
		useDaemonStore.setState({
			openers: [mine],
			tunnels: [buildTunnel({ open: { with: 'dbeaver' } })],
		});
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(screen.getByRole('button', { name: 'Supprimer DBeaver' }));

		await user.click(
			within(await screen.findByRole('alertdialog')).getByRole('button', {
				name: 'Supprimer',
			})
		);

		await waitFor(() => expect(gateway.updateOpeners).toHaveBeenCalledWith([]));
	});

	it('should keep the application when the confirmation is declined', async () => {
		useDaemonStore.setState({
			openers: [mine],
			tunnels: [buildTunnel({ open: { with: 'dbeaver' } })],
		});
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(screen.getByRole('button', { name: 'Supprimer DBeaver' }));

		await user.click(
			within(await screen.findByRole('alertdialog')).getByRole('button', {
				name: 'Annuler',
			})
		);

		expect(gateway.updateOpeners).not.toHaveBeenCalled();
	});

	it('should have no axe violations', async () => {
		useDaemonStore.setState({ openers: [preset, mine] });
		const { container } = renderSection();

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have no axe violations with the form open', async () => {
		const user = userEvent.setup();
		const { container } = renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une application' })
		);

		expect(await axe(container)).toHaveNoViolations();
	});
});
