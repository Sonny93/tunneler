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
			name: 'Configuration of nova',
		});
		expect(dialog).toHaveTextContent(
			'Effective configuration, equivalent to ssh -G nova.'
		);
		await screen.findByRole('table');
	});

	it('should show a busy state while loading', () => {
		renderDrawer();

		expect(screen.getByText('Reading the configuration…')).toBeInTheDocument();
	});

	it('should resolve the host of the panel', async () => {
		const { gateway } = renderDrawer();
		await screen.findByRole('table');

		expect(gateway.resolveHost).toHaveBeenCalledWith('nova');
	});

	it('should list the options in a captioned table', async () => {
		renderDrawer();

		const table = await screen.findByRole('table', {
			name: 'Effective options of nova',
		});
		expect(within(table).getByText('nova.example.com')).toBeInTheDocument();
		expect(
			within(table).getByRole('columnheader', { name: 'Option' })
		).toBeInTheDocument();
		expect(
			within(table).getByRole('columnheader', { name: 'Value' })
		).toBeInTheDocument();
	});

	it('should announce the number of options', async () => {
		renderDrawer();

		expect(await screen.findByRole('status')).toHaveTextContent(
			'3 options shown'
		);
	});

	it('should filter on key and value ignoring case', async () => {
		const user = userEvent.setup();
		renderDrawer();
		await screen.findByRole('table');

		await user.type(screen.getByLabelText('Filter options'), 'JULES');

		expect(screen.getByRole('status')).toHaveTextContent('1 option shown');
		expect(screen.queryByText('nova.example.com')).not.toBeInTheDocument();
	});

	it('should say so when no option matches', async () => {
		const user = userEvent.setup();
		renderDrawer();
		await screen.findByRole('table');

		await user.type(screen.getByLabelText('Filter options'), 'zzz');

		expect(screen.getByRole('status')).toHaveTextContent('No options shown');
		expect(screen.queryByRole('table')).not.toBeInTheDocument();
	});

	it('should copy the configuration as key value lines', async () => {
		const user = userEvent.setup();
		renderDrawer();
		await screen.findByRole('table');

		await user.click(
			screen.getByRole('button', { name: 'Copy the configuration' })
		);

		expect(await navigator.clipboard.readText()).toBe(
			'hostname nova.example.com\nuser jules\nport 22'
		);
		expect(
			useToastStore.getState().toasts.map((toast) => toast.message)
		).toEqual(['Configuration copied']);
	});

	it('should list the warnings in English without axe violations', async () => {
		renderDrawer({ ...RESOLVED_HOST, warnings: ['Include not found'] });

		const section = await screen.findByRole('region', {
			name: 'Warnings',
		});
		expect(within(section).getByText('Include not found')).toBeInTheDocument();
		expect(within(section).getByRole('list')).toHaveAttribute('lang', 'en');
		await screen.findByRole('table');
		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should show the error and retry', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.resolveHost.mockResolvedValue(RESOLVED_HOST);
		gateway.resolveHost.mockRejectedValueOnce(
			new DaemonCommandError('ssh_failed', 'ssh -G a failed')
		);
		useUiStore.getState().openHostConfig('nova');
		renderWithGateway(<HostConfigDrawer />, gateway);

		expect(await screen.findByRole('alert')).toHaveTextContent(
			"Couldn't read the configuration: ssh -G a failed"
		);
		await user.click(screen.getByRole('button', { name: 'Retry' }));

		expect(await screen.findByRole('table')).toBeInTheDocument();
		expect(gateway.resolveHost).toHaveBeenCalledTimes(2);
	});
});
