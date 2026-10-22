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

		const list = screen.getByRole('list', {
			name: 'Detected on this computer',
		});
		expect(within(list).getByText('Remmina (VNC)')).toBeInTheDocument();
		expect(
			within(list).getByText('remmina -c vnc://{host}:{port}')
		).toBeInTheDocument();
		expect(
			within(list).queryByRole('button', { name: /Edit|Delete/ })
		).not.toBeInTheDocument();
	});

	it('should offer a customize button named after each detected application', () => {
		useDaemonStore.setState({ openers: [preset] });
		renderSection();

		expect(
			screen.getByRole('button', { name: 'Customize Remmina (VNC)' })
		).toBeInTheDocument();
	});

	it('should open the applications guide in the active language from the hint', async () => {
		const user = userEvent.setup();
		const { gateway } = renderSection();

		await user.click(
			screen.getByRole('button', { name: 'How to add an application' })
		);

		expect(gateway.openDocumentation).toHaveBeenCalledWith(
			'applications',
			'en'
		);
	});

	it('should prefill the form from the detected application when customizing it', async () => {
		useDaemonStore.setState({ openers: [preset] });
		const user = userEvent.setup();
		renderSection();

		await user.click(
			screen.getByRole('button', { name: 'Customize Remmina (VNC)' })
		);

		const form = screen.getByRole('form', { name: 'Add an application' });
		expect(within(form).getByRole('textbox', { name: /^Name/ })).toHaveValue(
			'Remmina (VNC)'
		);
		expect(
			within(form).getByRole('textbox', { name: /^Identifier/ })
		).toHaveValue('remmina-vnc');
		expect(within(form).getByRole('textbox', { name: /^Command/ })).toHaveValue(
			'remmina'
		);
		expect(
			within(form).getByRole('textbox', { name: 'Arguments' })
		).toHaveValue('-c\nvnc://{host}:{port}');
		expect(
			within(form).getByRole('textbox', { name: /^Suggested ports/ })
		).toHaveValue('5900');
	});

	it('should say that the identifier replaces the detected application', async () => {
		useDaemonStore.setState({ openers: [preset] });
		const user = userEvent.setup();
		renderSection();

		await user.click(
			screen.getByRole('button', { name: 'Customize Remmina (VNC)' })
		);

		expect(
			screen.getByRole('textbox', { name: /^Identifier/ })
		).toHaveAccessibleDescription('Replaces the detected Remmina (VNC).');
	});

	it('should keep the identifier of the preset when its name is edited', async () => {
		useDaemonStore.setState({ openers: [preset] });
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Customize Remmina (VNC)' })
		);

		await user.type(screen.getByRole('textbox', { name: /^Name/ }), ' bis');

		expect(screen.getByRole('textbox', { name: /^Identifier/ })).toHaveValue(
			'remmina-vnc'
		);
	});

	it('should save the customized application with the identifier of the preset', async () => {
		useDaemonStore.setState({ openers: [preset, mine] });
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Customize Remmina (VNC)' })
		);
		const command = screen.getByRole('textbox', { name: /^Command/ });
		await user.clear(command);
		await user.type(command, 'C:\\Tools\\remmina.exe');

		await user.click(screen.getByRole('button', { name: 'Add' }));

		expect(gateway.updateOpeners).toHaveBeenCalledWith([
			MINE_SPEC,
			{
				id: 'remmina-vnc',
				name: 'Remmina (VNC)',
				command: 'C:\\Tools\\remmina.exe',
				args: ['-c', 'vnc://{host}:{port}'],
				ports: [5900],
			},
		]);
	});

	it('should spell out the parameters and the command in the field hints', async () => {
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);

		expect(
			screen.getByRole('textbox', { name: 'Arguments' })
		).toHaveAccessibleDescription(
			expect.stringContaining('the local port of the tunnel')
		);
		expect(
			screen.getByRole('textbox', { name: /^Command/ })
		).toHaveAccessibleDescription(expect.stringContaining('full path'));
	});

	it('should say when nothing is detected or added', () => {
		renderSection();

		expect(screen.getByText('No application detected.')).toBeInTheDocument();
		expect(screen.getByText('No application added.')).toBeInTheDocument();
	});

	it('should list the applications of the user with edit and delete buttons', () => {
		useDaemonStore.setState({ openers: [mine] });
		renderSection();

		expect(
			[
				screen.getByRole('button', { name: 'Edit DBeaver' }),
				screen.getByRole('button', { name: 'Delete DBeaver' }),
			].length
		).toBe(2);
	});

	it('should propose an identifier from the name while adding', async () => {
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);

		await user.type(
			screen.getByRole('textbox', { name: /^Name/ }),
			'My Client'
		);

		expect(screen.getByRole('textbox', { name: /^Identifier/ })).toHaveValue(
			'my-client'
		);
	});

	it('should stop following the name once the identifier is edited', async () => {
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Identifier/ }), 'x');

		await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'Other');

		expect(screen.getByRole('textbox', { name: /^Identifier/ })).toHaveValue(
			'x'
		);
	});

	it('should state that arguments are not interpreted by a shell', async () => {
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);

		expect(
			screen.getByRole('textbox', { name: 'Arguments' })
		).toHaveAccessibleDescription(
			expect.stringContaining('no shell interprets')
		);
	});

	it('should save the new application after the existing ones', async () => {
		useDaemonStore.setState({ openers: [mine] });
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'Viewer');
		await user.type(
			screen.getByRole('textbox', { name: /^Command/ }),
			'viewer'
		);
		await user.type(
			screen.getByRole('textbox', { name: 'Arguments' }),
			'--host={{host}{enter}--port={{port}'
		);
		await user.type(
			screen.getByRole('textbox', { name: /^Suggested ports/ }),
			'5900, 5901'
		);

		await user.click(screen.getByRole('button', { name: 'Add' }));

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
			screen.getByRole('button', { name: 'Add an application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'Viewer');
		await user.type(
			screen.getByRole('textbox', { name: /^Command/ }),
			'viewer'
		);

		await user.click(screen.getByRole('button', { name: 'Add' }));

		await waitFor(() =>
			expect(
				useToastStore.getState().toasts.map((toast) => toast.message)
			).toEqual(['Application Viewer added'])
		);
		expect(
			screen.getByRole('button', { name: 'Add an application' })
		).toBeInTheDocument();
	});

	it('should flag the invalid fields and not save', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);

		await user.click(screen.getByRole('button', { name: 'Add' }));

		expect(screen.getByRole('textbox', { name: /^Name/ })).toBeInvalid();
		expect(screen.getByRole('textbox', { name: /^Command/ })).toBeInvalid();
		expect(gateway.updateOpeners).not.toHaveBeenCalled();
	});

	it('should refuse an identifier already used by another application', async () => {
		useDaemonStore.setState({ openers: [mine] });
		const user = userEvent.setup();
		renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'DBeaver');
		await user.type(screen.getByRole('textbox', { name: /^Command/ }), 'x');

		await user.click(screen.getByRole('button', { name: 'Add' }));

		expect(
			screen.getByText(
				'This identifier is already used by one of your applications.'
			)
		).toBeInTheDocument();
	});

	it('should keep the identifier read only when editing', async () => {
		useDaemonStore.setState({ openers: [mine] });
		const user = userEvent.setup();
		renderSection();

		await user.click(screen.getByRole('button', { name: 'Edit DBeaver' }));

		expect(
			screen.getByRole('textbox', { name: /^Identifier/ })
		).toHaveAttribute('readonly');
	});

	it('should replace the edited application and keep the others', async () => {
		useDaemonStore.setState({ openers: [mine] });
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(screen.getByRole('button', { name: 'Edit DBeaver' }));
		const command = screen.getByRole('textbox', { name: /^Command/ });
		await user.clear(command);
		await user.type(command, '/opt/dbeaver');

		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(gateway.updateOpeners).toHaveBeenCalledWith([
			{ ...MINE_SPEC, command: '/opt/dbeaver' },
		]);
	});

	it('should show why the daemon refused the application', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.updateOpeners.mockRejectedValue(
			new DaemonCommandError('validation_failed', 'invalid', ['empty command'])
		);
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);
		await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'Viewer');
		await user.type(
			screen.getByRole('textbox', { name: /^Command/ }),
			'viewer'
		);

		await user.click(screen.getByRole('button', { name: 'Add' }));

		expect(await screen.findByRole('alert')).toHaveTextContent('empty command');
	});

	it('should cancel without saving', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);

		await user.click(screen.getByRole('button', { name: 'Cancel' }));

		expect(gateway.updateOpeners).not.toHaveBeenCalled();
		expect(
			screen.queryByRole('form', { name: 'Add an application' })
		).not.toBeInTheDocument();
	});

	it('should delete an unused application at once', async () => {
		useDaemonStore.setState({
			openers: [mine, buildOpener({ id: 'b', name: 'B', source: 'user' })],
		});
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		renderSection(gateway);

		await user.click(screen.getByRole('button', { name: 'Delete DBeaver' }));

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

		await user.click(screen.getByRole('button', { name: 'Delete DBeaver' }));

		const dialog = await screen.findByRole('alertdialog', {
			name: /Delete the application DBeaver/,
		});
		expect(dialog).toHaveTextContent('2 tunnels use this application');
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
		await user.click(screen.getByRole('button', { name: 'Delete DBeaver' }));

		await user.click(
			within(await screen.findByRole('alertdialog')).getByRole('button', {
				name: 'Delete',
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
		await user.click(screen.getByRole('button', { name: 'Delete DBeaver' }));

		await user.click(
			within(await screen.findByRole('alertdialog')).getByRole('button', {
				name: 'Cancel',
			})
		);

		expect(gateway.updateOpeners).not.toHaveBeenCalled();
	});

	it('should have no axe violations', async () => {
		useDaemonStore.setState({ openers: [preset, mine] });
		const { container } = renderSection();

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have no axe violations with the form customizing a detected application', async () => {
		useDaemonStore.setState({ openers: [preset] });
		const user = userEvent.setup();
		const { container } = renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Customize Remmina (VNC)' })
		);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should have no axe violations with the form open', async () => {
		const user = userEvent.setup();
		const { container } = renderSection();
		await user.click(
			screen.getByRole('button', { name: 'Add an application' })
		);

		expect(await axe(container)).toHaveNoViolations();
	});
});
