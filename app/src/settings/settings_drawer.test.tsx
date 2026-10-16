import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { screen, waitFor } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { buildSettings } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { SettingsDrawer } from '#/settings/settings_drawer';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';

function openSettings(settings = buildSettings()) {
	useDaemonStore.setState({ settings });
	useUiStore.getState().openSettings();
}

const renderDrawer = (gateway = createFakeGateway()) =>
	renderWithGateway(<SettingsDrawer />, gateway);

describe('SettingsDrawer', () => {
	it('should render nothing while closed', () => {
		renderDrawer();

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should say settings are unavailable without settings', () => {
		useUiStore.getState().openSettings();
		renderDrawer();

		expect(screen.getByText('Paramètres indisponibles.')).toBeInTheDocument();
	});

	it('should group the fields in labelled sections', () => {
		openSettings();
		renderDrawer();

		expect(
			['Adresses', 'Connexion', 'Reconnexion', 'Santé'].map(
				(name) => screen.queryByRole('group', { name }) !== null
			)
		).toEqual([true, true, true, true]);
	});

	it('should show durations in seconds', () => {
		openSettings(buildSettings({ keepaliveIntervalMs: 30_000 }));
		renderDrawer();

		expect(
			screen.getByRole('textbox', { name: /^Intervalle de keepalive/ })
		).toHaveValue('30');
	});

	it('should describe the health probe', () => {
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('switch', { name: 'Sonder la cible périodiquement' })
		).toHaveAccessibleDescription('Uniquement quand cette fenêtre est ouverte');
	});

	it('should have no axe violations', async () => {
		openSettings();
		renderDrawer();

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should save the edited settings in milliseconds and keep hidden fields', async () => {
		const settings = buildSettings({
			restoreOnLaunch: false,
			startOnLaunchTags: ['db'],
			launchAtLogin: true,
			x11Display: ':1',
		});
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		const grace = screen.getByRole('textbox', {
			name: /^Délai avant fermeture/,
		});

		await user.clear(grace);
		await user.type(grace, '90');
		await user.click(screen.getByRole('switch'));
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			sessionIdleGraceMs: 90_000,
			healthProbe: false,
		});
	});

	it('should close and toast after saving', async () => {
		openSettings();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		await waitFor(() => expect(useUiStore.getState().panel.kind).toBe('none'));
		expect(useToastStore.getState().toasts[0]?.message).toBe(
			'Paramètres enregistrés'
		);
	});

	it('should flag an invalid value and not save', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		const keepaliveMax = screen.getByRole('textbox', {
			name: /^Keepalives sans réponse/,
		});

		await user.clear(keepaliveMax);
		await user.type(keepaliveMax, 'x');
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(keepaliveMax).toHaveFocus();
		expect(keepaliveMax).toBeInvalid();
		expect(gateway.updateSettings).not.toHaveBeenCalled();
	});

	it('should show the daemon validation messages in a focused alert', async () => {
		openSettings();
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.updateSettings.mockRejectedValueOnce(
			new DaemonCommandError('validation_failed', 'invalide', [
				'Adresse invalide.',
			])
		);
		renderDrawer(gateway);

		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent('Adresse invalide.');
		expect(alert).toHaveFocus();
	});
});
