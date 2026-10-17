import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { screen, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import type { ResolvedHostDto } from '#/daemon/protocol';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { HostConfigDrawer } from '#/host_config/host_config_drawer';

const RESOLVED_HOST: ResolvedHostDto = {
	host: 'nova',
	options: [
		{ key: 'hostname', value: 'nova.example.com' },
		{ key: 'user', value: 'jules' },
		{ key: 'port', value: '22' },
	],
	warnings: [],
};

function renderDrawer(resolvedHost = RESOLVED_HOST) {
	const gateway = createFakeGateway();
	gateway.resolveHost.mockResolvedValue(resolvedHost);
	useUiStore.getState().openHostConfig('nova');
	return renderWithGateway(<HostConfigDrawer />, gateway);
}

describe('HostConfigDrawer', () => {
	it('should render nothing while another panel is open', () => {
		useUiStore.getState().openSettings();
		renderWithGateway(<HostConfigDrawer />);

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should title the dialog with the host and explain the content', async () => {
		renderDrawer();

		const dialog = screen.getByRole('dialog', {
			name: 'Configuration de nova',
		});
		expect(dialog).toHaveTextContent(
			'Configuration effective, équivalent de ssh -G nova.'
		);
		await screen.findByRole('table');
	});

	it('should show a busy state while loading', () => {
		renderDrawer();

		expect(
			screen.getByText('Lecture de la configuration…')
		).toBeInTheDocument();
	});

	it('should resolve the host of the panel', async () => {
		const { gateway } = renderDrawer();
		await screen.findByRole('table');

		expect(gateway.resolveHost).toHaveBeenCalledWith('nova');
	});

	it('should list the options in a captioned table', async () => {
		renderDrawer();

		const table = await screen.findByRole('table', {
			name: 'Options effectives de nova',
		});
		expect(within(table).getByText('nova.example.com')).toBeInTheDocument();
		expect(
			within(table).getByRole('columnheader', { name: 'Option' })
		).toBeInTheDocument();
		expect(
			within(table).getByRole('columnheader', { name: 'Valeur' })
		).toBeInTheDocument();
	});

	it('should announce the number of options', async () => {
		renderDrawer();

		expect(await screen.findByRole('status')).toHaveTextContent(
			'3 options affichées'
		);
	});

	it('should filter on key and value ignoring case', async () => {
		const user = userEvent.setup();
		renderDrawer();
		await screen.findByRole('table');

		await user.type(screen.getByLabelText('Filtrer les options'), 'JULES');

		expect(screen.getByRole('status')).toHaveTextContent('1 option affichée');
		expect(screen.queryByText('nova.example.com')).not.toBeInTheDocument();
	});

	it('should say so when no option matches', async () => {
		const user = userEvent.setup();
		renderDrawer();
		await screen.findByRole('table');

		await user.type(screen.getByLabelText('Filtrer les options'), 'zzz');

		expect(screen.getByRole('status')).toHaveTextContent(
			'Aucune option affichée'
		);
		expect(screen.queryByRole('table')).not.toBeInTheDocument();
	});

	it('should copy the configuration as key value lines', async () => {
		const user = userEvent.setup();
		renderDrawer();
		await screen.findByRole('table');

		await user.click(
			screen.getByRole('button', { name: 'Copier la configuration' })
		);

		expect(await navigator.clipboard.readText()).toBe(
			'hostname nova.example.com\nuser jules\nport 22'
		);
		expect(
			useToastStore.getState().toasts.map((toast) => toast.message)
		).toEqual(['Configuration copiée']);
	});

	it('should list the warnings in English', async () => {
		renderDrawer({ ...RESOLVED_HOST, warnings: ['Include not found'] });

		const section = await screen.findByRole('region', {
			name: 'Avertissements',
		});
		expect(within(section).getByText('Include not found')).toBeInTheDocument();
		expect(within(section).getByRole('list')).toHaveAttribute('lang', 'en');
	});

	it('should show the error and retry', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.resolveHost.mockResolvedValue(RESOLVED_HOST);
		gateway.resolveHost.mockRejectedValueOnce(
			new DaemonCommandError('ssh_failed', 'ssh -G a échoué')
		);
		useUiStore.getState().openHostConfig('nova');
		renderWithGateway(<HostConfigDrawer />, gateway);

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'Lecture impossible : ssh -G a échoué'
		);
		await user.click(screen.getByRole('button', { name: 'Réessayer' }));

		expect(await screen.findByRole('table')).toBeInTheDocument();
		expect(gateway.resolveHost).toHaveBeenCalledTimes(2);
	});

	it('should have no axe violations', async () => {
		renderDrawer({ ...RESOLVED_HOST, warnings: ['Include not found'] });
		await screen.findByRole('table');

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
