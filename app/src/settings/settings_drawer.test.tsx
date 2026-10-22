import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { screen, waitFor, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { SettingsDrawer } from '#/settings/settings_drawer';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';

const renderDrawerWithModals = (gateway = createFakeGateway()) =>
	renderWithGateway(
		<>
			<ModalProvider />
			<SettingsDrawer />
		</>,
		gateway
	);

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
			[
				'Adresses',
				'Connexion',
				'Reconnexion',
				'X11',
				'Veille',
				'Ouverture',
				'Démarrage',
				'Secrets',
				'Mises à jour',
			].map((name) => screen.queryByRole('group', { name }) !== null)
		).toEqual([true, true, true, true, true, true, true, true, true]);
	});

	it('should show durations in seconds', () => {
		openSettings(buildSettings({ keepaliveIntervalMs: 30_000 }));
		renderDrawer();

		expect(
			screen.getByRole('textbox', { name: /^Intervalle de keepalive/ })
		).toHaveValue('30');
	});

	it('should have no axe violations', async () => {
		openSettings();
		renderDrawer();

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should save the edited settings in milliseconds', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		const grace = screen.getByRole('textbox', {
			name: /^Délai avant fermeture/,
		});

		await user.clear(grace);
		await user.type(grace, '90');
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			sessionIdleGraceMs: 90_000,
		});
	});

	it('should offer the applications section', () => {
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('region', { name: 'Applications' })
		).toBeInTheDocument();
	});

	it('should show the current open host', () => {
		openSettings(buildSettings({ openHost: 'ip' }));
		renderDrawer();

		expect(screen.getByRole('radio', { name: /^Adresse IP/ })).toBeChecked();
	});

	it('should offer the standby switch and delay in a labelled section', () => {
		openSettings();
		renderDrawer();

		const section = screen.getByRole('group', { name: 'Veille' });
		expect(
			within(section).getByRole('switch', {
				name: 'Mettre les tunnels en veille',
			})
		).not.toBeChecked();
		expect(
			within(section).getByRole('textbox', {
				name: /^Retour en veille après \(secondes\)/,
			})
		).toHaveValue('300');
	});

	it('should save the standby switch and delay', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		const delay = screen.getByRole('textbox', {
			name: /^Retour en veille après/,
		});

		await user.click(
			screen.getByRole('switch', { name: 'Mettre les tunnels en veille' })
		);
		await user.clear(delay);
		await user.type(delay, '120');
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			standby: true,
			standbyIdleS: 120,
		});
	});

	it('should offer the standby signal choices with the current one selected', () => {
		openSettings(buildSettings({ standbyNotify: 'sound' }));
		renderDrawer();

		const select = screen.getByRole('combobox', {
			name: 'Signal à la sortie de veille',
		});
		expect(
			within(select)
				.getAllByRole('option')
				.map((option) => option.getAttribute('value'))
		).toEqual(['none', 'visual', 'sound', 'both']);
		expect(select).toHaveValue('sound');
	});

	it('should say when the standby signal is given', () => {
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('combobox', { name: 'Signal à la sortie de veille' })
		).toHaveAccessibleDescription(/quand le réveil échoue/);
	});

	it('should save the chosen standby signal', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Signal à la sortie de veille' }),
			'Les deux'
		);
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			standbyNotify: 'both',
		});
	});

	it('should save the chosen standby sound', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Son' }),
			'Marimba'
		);
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			standbySound: 'marimba',
		});
	});

	it('should preview the unsaved standby sound', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Son' }),
			'Carillon'
		);
		await user.click(screen.getByRole('button', { name: 'Écouter le réveil' }));

		expect(gateway.previewSound).toHaveBeenCalledWith('chime', 'woke');
	});

	it('should flag an invalid standby delay and not save', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		const delay = screen.getByRole('textbox', {
			name: /^Retour en veille après/,
		});

		await user.clear(delay);
		await user.type(delay, '0');
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(delay).toBeInvalid();
		expect(gateway.updateSettings).not.toHaveBeenCalled();
	});

	it('should save the chosen open host', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('radio', { name: /^Adresse IP/ }));
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			openHost: 'ip',
		});
	});

	it('should describe the X11 display field', () => {
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('textbox', { name: 'Display X11 local' })
		).toHaveAccessibleDescription(
			'Vide : variable DISPLAY (127.0.0.1:0.0 sous Windows)'
		);
	});

	it('should save the edited X11 display', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.type(
			screen.getByRole('textbox', { name: 'Display X11 local' }),
			'203.0.113.20:0.0'
		);
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ x11Display: '203.0.113.20:0.0' })
		);
	});

	it('should save an emptied X11 display as unset', async () => {
		openSettings(buildSettings({ x11Display: ':1' }));
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.clear(
			screen.getByRole('textbox', { name: 'Display X11 local' })
		);
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ x11Display: undefined })
		);
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

	it('should describe the start-up switches', () => {
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('switch', {
				name: "Lancer Tunneler à l'ouverture de session",
			})
		).toHaveAccessibleDescription(
			'Démarre le daemon et son icône, sans ouvrir la fenêtre.'
		);
	});

	it('should list the existing tags in the start-up tags hint', () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({ id: 'a', tags: ['dev'] }),
				buildTunnel({ id: 'b', tags: ['adonis', 'dev'] }),
			],
		});
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('textbox', { name: 'Tags démarrés au lancement' })
		).toHaveAccessibleDescription(
			'Séparés par des virgules. Tags existants : adonis, dev.'
		);
	});

	it('should save the toggled start-up and secrets switches', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(
			screen.getByRole('switch', {
				name: "Lancer Tunneler à l'ouverture de session",
			})
		);
		await user.click(
			screen.getByRole('switch', {
				name: 'Rétablir les tunnels actifs au lancement',
			})
		);
		await user.click(
			screen.getByRole('switch', {
				name: 'Mémoriser les passphrases dans le trousseau du système',
			})
		);
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			launchAtLogin: true,
			restoreOnLaunch: false,
			useOsKeyring: true,
		});
	});

	it('should describe that checking for updates contacts GitHub', () => {
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('switch', {
				name: "Vérifier les mises à jour à l'ouverture",
			})
		).toHaveAccessibleDescription(/contacte GitHub/);
	});

	it('should save the check for updates switch once turned off', async () => {
		const settings = buildSettings({ checkForUpdates: true });
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(
			screen.getByRole('switch', {
				name: "Vérifier les mises à jour à l'ouverture",
			})
		);
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			checkForUpdates: false,
		});
	});

	it('should save the start-up tags parsed from the text', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.type(
			screen.getByRole('textbox', { name: 'Tags démarrés au lancement' }),
			'dev, adonis, dev'
		);
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ startOnLaunchTags: ['dev', 'adonis'] })
		);
	});

	describe('forgetting remembered passphrases', () => {
		const clickForget = async (user: ReturnType<typeof userEvent.setup>) =>
			user.click(
				screen.getByRole('button', {
					name: 'Oublier les passphrases mémorisées',
				})
			);

		it('should ask for confirmation before forgetting', async () => {
			openSettings();
			const user = userEvent.setup();
			const { gateway } = renderDrawerWithModals();

			await clickForget(user);

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Oublier les passphrases mémorisées ?',
				})
			).toBeInTheDocument();
			expect(gateway.forgetSecrets).not.toHaveBeenCalled();
		});

		it('should not forget when cancelled', async () => {
			openSettings();
			const user = userEvent.setup();
			const { gateway } = renderDrawerWithModals();
			await clickForget(user);

			const dialog = await screen.findByRole('alertdialog');
			await user.click(within(dialog).getByRole('button', { name: 'Annuler' }));

			expect(gateway.forgetSecrets).not.toHaveBeenCalled();
		});

		it('should forget and toast the count when confirmed', async () => {
			openSettings();
			const user = userEvent.setup();
			const gateway = createFakeGateway();
			gateway.forgetSecrets.mockResolvedValue({ forgottenCount: 3 });
			renderDrawerWithModals(gateway);
			await clickForget(user);

			await user.click(await screen.findByRole('button', { name: 'Oublier' }));

			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]?.message).toBe(
					'3 passphrases oubliées'
				)
			);
		});

		it('should toast an error when forgetting fails', async () => {
			openSettings();
			const user = userEvent.setup();
			const gateway = createFakeGateway();
			gateway.forgetSecrets.mockRejectedValue(
				new DaemonCommandError('internal', 'trousseau verrouillé')
			);
			renderDrawerWithModals(gateway);
			await clickForget(user);

			await user.click(await screen.findByRole('button', { name: 'Oublier' }));

			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]).toMatchObject({
					tone: 'error',
					message: 'Oubli impossible : trousseau verrouillé',
				})
			);
		});
	});
});
