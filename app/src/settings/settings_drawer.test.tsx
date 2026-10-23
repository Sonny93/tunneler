import { ModalProvider } from '@minimalstuff/ui';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { activateLocale } from '#/i18n/activate_locale';
import { SettingsDrawer } from '#/settings/settings_drawer';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';

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

		expect(screen.getByText('Settings unavailable.')).toBeInTheDocument();
	});

	it('should group the fields in labelled sections', () => {
		openSettings();
		renderDrawer();

		expect(
			[
				'Language',
				'Addresses',
				'Connection',
				'Reconnection',
				'X11',
				'Standby',
				'Opening',
				'Startup',
				'Secrets',
				'Updates',
			].map((name) => screen.queryByRole('group', { name }) !== null)
		).toEqual([true, true, true, true, true, true, true, true, true, true]);
	});

	it('should show durations in seconds', () => {
		openSettings(buildSettings({ keepaliveIntervalMs: 30_000 }));
		renderDrawer();

		expect(
			screen.getByRole('textbox', { name: /^Keepalive interval/ })
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
			name: /^Time before an unused session/,
		});

		await user.clear(grace);
		await user.type(grace, '90');
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			sessionIdleGraceMs: 90_000,
		});
	});

	it('should offer the display languages with the current one selected', () => {
		openSettings(buildSettings({ language: 'fr' }));
		renderDrawer();

		const select = screen.getByRole('combobox', { name: 'Display language' });
		expect(
			within(select)
				.getAllByRole('option')
				.map((option) => option.getAttribute('value'))
		).toEqual(['system', 'en', 'fr']);
		expect(select).toHaveValue('fr');
	});

	it('should save the chosen language', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Display language' }),
			'Français'
		);
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			language: 'fr',
		});
		expect(useDaemonStore.getState().settings?.language).toBe('fr');
	});

	it('should display the settings in French when the French locale is active', async () => {
		await activateLocale('fr');
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('combobox', { name: "Langue d'affichage" })
		).toBeInTheDocument();
		expect(
			screen.getByRole('button', { name: 'Enregistrer' })
		).toBeInTheDocument();
		expect(
			screen.getByRole('switch', { name: 'Mettre les tunnels en veille' })
		).toBeInTheDocument();
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

		expect(screen.getByRole('radio', { name: /^IP address/ })).toBeChecked();
	});

	it('should offer the standby switch and delay in a labelled section', () => {
		openSettings();
		renderDrawer();

		const section = screen.getByRole('group', { name: 'Standby' });
		expect(
			within(section).getByRole('switch', {
				name: 'Put tunnels on standby',
			})
		).not.toBeChecked();
		expect(
			within(section).getByRole('textbox', {
				name: /^Return to standby after \(seconds\)/,
			})
		).toHaveValue('300');
	});

	it('should save the standby switch and delay', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		const delay = screen.getByRole('textbox', {
			name: /^Return to standby after/,
		});

		await user.click(
			screen.getByRole('switch', { name: 'Put tunnels on standby' })
		);
		await user.clear(delay);
		await user.type(delay, '120');
		await user.click(screen.getByRole('button', { name: 'Save' }));

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
			name: 'Signal when leaving standby',
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
			screen.getByRole('combobox', { name: 'Signal when leaving standby' })
		).toHaveAccessibleDescription(/when waking fails/);
	});

	it('should save the chosen standby signal', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Signal when leaving standby' }),
			'Both'
		);
		await user.click(screen.getByRole('button', { name: 'Save' }));

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
			screen.getByRole('combobox', { name: 'Sound' }),
			'Marimba'
		);
		await user.click(screen.getByRole('button', { name: 'Save' }));

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
			screen.getByRole('combobox', { name: 'Sound' }),
			'Chime'
		);
		await user.click(
			screen.getByRole('button', { name: 'Play the wake sound' })
		);

		expect(gateway.previewSound).toHaveBeenCalledWith('chime', 'woke');
	});

	it('should flag an invalid standby delay and not save', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		const delay = screen.getByRole('textbox', {
			name: /^Return to standby after/,
		});

		await user.clear(delay);
		await user.type(delay, '0');
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(delay).toBeInvalid();
		expect(gateway.updateSettings).not.toHaveBeenCalled();
	});

	it('should save the chosen open host', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('radio', { name: /^IP address/ }));
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith({
			...settings,
			openHost: 'ip',
		});
	});

	it('should describe the X11 display field', () => {
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('textbox', { name: 'Local X11 display' })
		).toHaveAccessibleDescription(
			'Empty: DISPLAY variable (127.0.0.1:0.0 on Windows)'
		);
	});

	it('should save the edited X11 display', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.type(
			screen.getByRole('textbox', { name: 'Local X11 display' }),
			'203.0.113.20:0.0'
		);
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ x11Display: '203.0.113.20:0.0' })
		);
	});

	it('should save an emptied X11 display as unset', async () => {
		openSettings(buildSettings({ x11Display: ':1' }));
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.clear(
			screen.getByRole('textbox', { name: 'Local X11 display' })
		);
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ x11Display: undefined })
		);
	});

	it('should close and toast after saving', async () => {
		openSettings();
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() => expect(useUiStore.getState().panel.kind).toBe('none'));
		expect(useToastStore.getState().toasts[0]?.message).toBe('Settings saved');
	});

	it('should flag an invalid value and not save', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		const keepaliveMax = screen.getByRole('textbox', {
			name: /^Unanswered keepalives/,
		});

		await user.clear(keepaliveMax);
		await user.type(keepaliveMax, 'x');
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(keepaliveMax).toHaveFocus();
		expect(keepaliveMax).toBeInvalid();
		expect(gateway.updateSettings).not.toHaveBeenCalled();
	});

	it('should show the daemon validation messages in a focused alert', async () => {
		openSettings();
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.updateSettings.mockRejectedValueOnce(
			new DaemonCommandError('validation_failed', 'invalid', [
				'Invalid address.',
			])
		);
		renderDrawer(gateway);

		await user.click(screen.getByRole('button', { name: 'Save' }));

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent('Invalid address.');
		expect(alert).toHaveFocus();
	});

	it('should describe the start-up switches', () => {
		openSettings();
		renderDrawer();

		expect(
			screen.getByRole('switch', {
				name: 'Launch Tunneler at login',
			})
		).toHaveAccessibleDescription(
			'Starts the daemon and its tray icon without opening the window.'
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
			screen.getByRole('textbox', { name: 'Tags to start on launch' })
		).toHaveAccessibleDescription(
			'Separated by commas. Existing tags: adonis, dev.'
		);
	});

	it('should save the toggled start-up and secrets switches', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(
			screen.getByRole('switch', {
				name: 'Launch Tunneler at login',
			})
		);
		await user.click(
			screen.getByRole('switch', {
				name: 'Restore active tunnels on launch',
			})
		);
		await user.click(
			screen.getByRole('switch', {
				name: 'Remember passphrases in the system keyring',
			})
		);
		await user.click(screen.getByRole('button', { name: 'Save' }));

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
				name: 'Check for updates when opening',
			})
		).toHaveAccessibleDescription(/contacts GitHub/);
	});

	it('should save the check for updates switch once turned off', async () => {
		const settings = buildSettings({ checkForUpdates: true });
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(
			screen.getByRole('switch', {
				name: 'Check for updates when opening',
			})
		);
		await user.click(screen.getByRole('button', { name: 'Save' }));

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
			screen.getByRole('textbox', { name: 'Tags to start on launch' }),
			'dev, adonis, dev'
		);
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ startOnLaunchTags: ['dev', 'adonis'] })
		);
	});

	describe('forgetting remembered passphrases', () => {
		const clickForget = async (user: ReturnType<typeof userEvent.setup>) =>
			user.click(
				screen.getByRole('button', {
					name: 'Forget remembered passphrases',
				})
			);

		it('should ask for confirmation before forgetting', async () => {
			openSettings();
			const user = userEvent.setup();
			const { gateway } = renderDrawerWithModals();

			await clickForget(user);

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Forget remembered passphrases?',
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
			await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));

			expect(gateway.forgetSecrets).not.toHaveBeenCalled();
		});

		it('should forget and toast the count when confirmed', async () => {
			openSettings();
			const user = userEvent.setup();
			const gateway = createFakeGateway();
			gateway.forgetSecrets.mockResolvedValue({ forgottenCount: 3 });
			renderDrawerWithModals(gateway);
			await clickForget(user);

			await user.click(await screen.findByRole('button', { name: 'Forget' }));

			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]?.message).toBe(
					'3 passphrases forgotten'
				)
			);
		});

		it('should toast an error when forgetting fails', async () => {
			openSettings();
			const user = userEvent.setup();
			const gateway = createFakeGateway();
			gateway.forgetSecrets.mockRejectedValue(
				new DaemonCommandError('internal', 'keyring locked')
			);
			renderDrawerWithModals(gateway);
			await clickForget(user);

			await user.click(await screen.findByRole('button', { name: 'Forget' }));

			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]).toMatchObject({
					tone: 'error',
					message: "Couldn't forget the passphrases: Internal error",
				})
			);
		});
	});
});
