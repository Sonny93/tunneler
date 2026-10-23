import { ModalProvider } from '@minimalstuff/ui';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { activateLocale } from '#/i18n/activate_locale';
import { SettingsDialogHost } from '#/settings/settings_dialog_host';
import type { SettingsTab } from '#/settings/settings_tab';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import {
	buildOpener,
	buildOpenerSpec,
	buildSettings,
	buildTunnel,
} from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';

type User = ReturnType<typeof userEvent.setup>;

function openSettings(settings = buildSettings(), tab?: SettingsTab) {
	useDaemonStore.setState({ settings });
	useUiStore.getState().openSettings(tab);
}

async function renderDialog(gateway = createFakeGateway()) {
	const rendered = renderWithGateway(
		<>
			<ModalProvider />
			<SettingsDialogHost />
		</>,
		gateway
	);
	await screen.findByRole('dialog', { name: /^(Settings|Paramètres)$/ });
	return rendered;
}

const selectTab = (user: User, name: string | RegExp) =>
	user.click(screen.getByRole('tab', { name }));

const clickSave = (user: User) =>
	user.click(screen.getByRole('button', { name: 'Save' }));

const getTab = (name: string | RegExp) => screen.getByRole('tab', { name });

describe('SettingsDialog', () => {
	it('should render nothing while closed', async () => {
		renderWithGateway(
			<>
				<ModalProvider />
				<SettingsDialogHost />
			</>
		);

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should say settings are unavailable without settings', async () => {
		useUiStore.getState().openSettings();
		await renderDialog();

		expect(screen.getByText('Settings unavailable.')).toBeInTheDocument();
	});

	it('should open on the general tab by default', async () => {
		openSettings();
		await renderDialog();

		expect(getTab('General')).toHaveAttribute('aria-selected', 'true');
	});

	it('should open on the requested tab', async () => {
		openSettings(buildSettings(), 'standby');
		await renderDialog();

		expect(getTab('Standby')).toHaveAttribute('aria-selected', 'true');
		expect(
			screen.getByRole('switch', { name: 'Put tunnels on standby' })
		).toBeVisible();
	});

	it('should lay the tabs out vertically', async () => {
		openSettings();
		await renderDialog();

		expect(screen.getByRole('tablist')).toHaveAttribute(
			'aria-orientation',
			'vertical'
		);
	});

	it('should move between the tabs with the vertical arrow keys', async () => {
		openSettings();
		const user = userEvent.setup();
		await renderDialog();
		getTab('General').focus();

		await user.keyboard('{ArrowDown}');

		expect(getTab('Connection')).toHaveFocus();
		expect(getTab('Connection')).toHaveAttribute('aria-selected', 'true');
	});

	it('should end the dialog when the panel is closed', async () => {
		openSettings();
		await renderDialog();

		useUiStore.getState().closePanel();

		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
	});

	it('should close the panel when the dialog is dismissed with Escape', async () => {
		openSettings();
		const user = userEvent.setup();
		await renderDialog();

		await user.keyboard('{Escape}');

		await waitFor(() => expect(useUiStore.getState().panel.kind).toBe('none'));
	});

	it.each([
		['General', ['Language', 'Startup', 'Updates']],
		['Connection', ['Addresses', 'Sessions', 'Reconnection', 'X11']],
		['Standby', ['Standby']],
		['Opening', ['Opening']],
		['Secrets', ['Secrets']],
	])('should group the %s fields in labelled sections', async (tab, groups) => {
		openSettings();
		const user = userEvent.setup();
		await renderDialog();

		await selectTab(user, tab);

		expect(
			groups.map((name) => screen.queryByRole('group', { name }) !== null)
		).toEqual(groups.map(() => true));
	});

	it.each(['General', 'Connection', 'Standby', 'Opening', 'Secrets'])(
		'should have no axe violations on the %s tab',
		async (tab) => {
			openSettings();
			const user = userEvent.setup();
			await renderDialog();
			await selectTab(user, tab);

			expect(await axe(document.body)).toHaveNoViolations();
		}
	);

	it('should show durations in seconds', async () => {
		openSettings(buildSettings({ keepaliveIntervalMs: 30_000 }), 'connection');
		await renderDialog();

		expect(
			screen.getByRole('textbox', { name: /^Keepalive interval/ })
		).toHaveValue('30');
	});

	it('should keep a typed value when switching tabs', async () => {
		openSettings(buildSettings(), 'connection');
		const user = userEvent.setup();
		await renderDialog();
		const address = screen.getByRole('textbox', {
			name: /^Default listen address \(local\)/,
		});
		await user.clear(address);
		await user.type(address, '0.0.0.0');

		await selectTab(user, 'Secrets');
		await selectTab(user, 'Connection');

		expect(
			screen.getByRole('textbox', { name: /^Default listen address \(local\)/ })
		).toHaveValue('0.0.0.0');
	});

	it('should save the edited settings in milliseconds with the applications', async () => {
		const settings = buildSettings();
		openSettings(settings, 'connection');
		const user = userEvent.setup();
		const { gateway } = await renderDialog();
		const grace = screen.getByRole('textbox', {
			name: /^Time before an unused session/,
		});

		await user.clear(grace);
		await user.type(grace, '90');
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			{ ...settings, sessionIdleGraceMs: 90_000 },
			[]
		);
	});

	it('should offer the display languages with the current one selected', async () => {
		openSettings(buildSettings({ language: 'fr' }));
		await renderDialog();

		const select = screen.getByRole('combobox', { name: 'Display language' });
		expect(
			within(select)
				.getAllByRole('option')
				.map((option) => option.getAttribute('value'))
		).toEqual(['system', 'en', 'fr']);
		expect(select).toHaveValue('fr');
	});

	it('should save the chosen language and update the store', async () => {
		const settings = buildSettings();
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Display language' }),
			'Français'
		);
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			{ ...settings, language: 'fr' },
			[]
		);
		expect(useDaemonStore.getState().settings?.language).toBe('fr');
	});

	it('should display the settings in French when the French locale is active', async () => {
		await activateLocale('fr');
		openSettings();
		await renderDialog();

		expect(
			screen.getByRole('combobox', { name: "Langue d'affichage" })
		).toBeInTheDocument();
		expect(
			screen.getByRole('button', { name: 'Enregistrer' })
		).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Veille' })).toBeInTheDocument();
	});

	it('should show the current open host', async () => {
		openSettings(buildSettings({ openHost: 'ip' }), 'opening');
		await renderDialog();

		expect(screen.getByRole('radio', { name: /^IP address/ })).toBeChecked();
	});

	it('should offer the applications section on the opening tab', async () => {
		openSettings(buildSettings(), 'opening');
		await renderDialog();

		expect(
			screen.getByRole('region', { name: 'Applications' })
		).toBeInTheDocument();
	});

	it('should save the chosen open host', async () => {
		const settings = buildSettings();
		openSettings(settings, 'opening');
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.click(screen.getByRole('radio', { name: /^IP address/ }));
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			{ ...settings, openHost: 'ip' },
			[]
		);
	});

	it('should offer the standby switch and delay in a labelled section', async () => {
		openSettings(buildSettings(), 'standby');
		await renderDialog();

		const section = screen.getByRole('group', { name: 'Standby' });
		expect(
			within(section).getByRole('switch', { name: 'Put tunnels on standby' })
		).not.toBeChecked();
		expect(
			within(section).getByRole('textbox', {
				name: /^Return to standby after \(seconds\)/,
			})
		).toHaveValue('300');
	});

	it('should save the standby switch and delay', async () => {
		const settings = buildSettings();
		openSettings(settings, 'standby');
		const user = userEvent.setup();
		const { gateway } = await renderDialog();
		const delay = screen.getByRole('textbox', {
			name: /^Return to standby after/,
		});

		await user.click(
			screen.getByRole('switch', { name: 'Put tunnels on standby' })
		);
		await user.clear(delay);
		await user.type(delay, '120');
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			{ ...settings, standby: true, standbyIdleS: 120 },
			[]
		);
	});

	it('should offer the standby signal choices with the current one selected', async () => {
		openSettings(buildSettings({ standbyNotify: 'sound' }), 'standby');
		await renderDialog();

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

	it('should say when the standby signal is given', async () => {
		openSettings(buildSettings(), 'standby');
		await renderDialog();

		expect(
			screen.getByRole('combobox', { name: 'Signal when leaving standby' })
		).toHaveAccessibleDescription(/when waking fails/);
	});

	it('should save the chosen standby signal', async () => {
		const settings = buildSettings();
		openSettings(settings, 'standby');
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Signal when leaving standby' }),
			'Both'
		);
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			{ ...settings, standbyNotify: 'both' },
			[]
		);
	});

	it('should save the chosen standby sound', async () => {
		const settings = buildSettings();
		openSettings(settings, 'standby');
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Sound' }),
			'Marimba'
		);
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			{ ...settings, standbySound: 'marimba' },
			[]
		);
	});

	it('should preview the unsaved standby sound without saving', async () => {
		openSettings(buildSettings(), 'standby');
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.selectOptions(
			screen.getByRole('combobox', { name: 'Sound' }),
			'Chime'
		);
		await user.click(
			screen.getByRole('button', { name: 'Play the wake sound' })
		);

		expect(gateway.previewSound).toHaveBeenCalledWith('chime', 'woke');
		expect(gateway.updateSettings).not.toHaveBeenCalled();
	});

	it('should describe the X11 display field', async () => {
		openSettings(buildSettings(), 'connection');
		await renderDialog();

		expect(
			screen.getByRole('textbox', { name: 'Local X11 display' })
		).toHaveAccessibleDescription(
			'Empty: DISPLAY variable (127.0.0.1:0.0 on Windows)'
		);
	});

	it('should save the edited X11 display', async () => {
		openSettings(buildSettings(), 'connection');
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.type(
			screen.getByRole('textbox', { name: 'Local X11 display' }),
			'203.0.113.20:0.0'
		);
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ x11Display: '203.0.113.20:0.0' }),
			[]
		);
	});

	it('should save an emptied X11 display as unset', async () => {
		openSettings(buildSettings({ x11Display: ':1' }), 'connection');
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.clear(
			screen.getByRole('textbox', { name: 'Local X11 display' })
		);
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ x11Display: undefined }),
			[]
		);
	});

	it('should close and toast after saving', async () => {
		openSettings();
		const user = userEvent.setup();
		await renderDialog();

		await clickSave(user);

		await waitFor(() => expect(useUiStore.getState().panel.kind).toBe('none'));
		expect(useToastStore.getState().toasts[0]?.message).toBe('Settings saved');
		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
	});

	it('should describe the start-up switches', async () => {
		openSettings();
		await renderDialog();

		expect(
			screen.getByRole('switch', { name: 'Launch Tunneler at login' })
		).toHaveAccessibleDescription(
			'Starts the daemon and its tray icon without opening the window.'
		);
	});

	it('should list the existing tags in the start-up tags hint', async () => {
		useDaemonStore.setState({
			tunnels: [
				buildTunnel({ id: 'a', tags: ['dev'] }),
				buildTunnel({ id: 'b', tags: ['adonis', 'dev'] }),
			],
		});
		openSettings();
		await renderDialog();

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
		const { gateway } = await renderDialog();

		await user.click(
			screen.getByRole('switch', { name: 'Launch Tunneler at login' })
		);
		await user.click(
			screen.getByRole('switch', { name: 'Restore active tunnels on launch' })
		);
		await selectTab(user, 'Secrets');
		await user.click(
			screen.getByRole('switch', {
				name: 'Remember passphrases in the system keyring',
			})
		);
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			{
				...settings,
				launchAtLogin: true,
				restoreOnLaunch: false,
				useOsKeyring: true,
			},
			[]
		);
	});

	it('should describe that checking for updates contacts GitHub', async () => {
		openSettings();
		await renderDialog();

		expect(
			screen.getByRole('switch', { name: 'Check for updates when opening' })
		).toHaveAccessibleDescription(/contacts GitHub/);
	});

	it('should save the check for updates switch once turned off', async () => {
		const settings = buildSettings({ checkForUpdates: true });
		openSettings(settings);
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.click(
			screen.getByRole('switch', { name: 'Check for updates when opening' })
		);
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			{ ...settings, checkForUpdates: false },
			[]
		);
	});

	it('should save the start-up tags parsed from the text', async () => {
		openSettings();
		const user = userEvent.setup();
		const { gateway } = await renderDialog();

		await user.type(
			screen.getByRole('textbox', { name: 'Tags to start on launch' }),
			'dev, adonis, dev'
		);
		await clickSave(user);

		expect(gateway.updateSettings).toHaveBeenCalledWith(
			expect.objectContaining({ startOnLaunchTags: ['dev', 'adonis'] }),
			[]
		);
	});

	describe('validation', () => {
		async function submitInvalidKeepaliveMax(
			user: User,
			gateway = createFakeGateway()
		) {
			openSettings(buildSettings(), 'connection');
			const rendered = await renderDialog(gateway);
			const keepaliveMax = screen.getByRole('textbox', {
				name: /^Unanswered keepalives/,
			});
			await user.clear(keepaliveMax);
			await user.type(keepaliveMax, 'x');
			await selectTab(user, 'General');
			await clickSave(user);
			return rendered;
		}

		it('should activate the tab of the first invalid field and focus the field', async () => {
			const user = userEvent.setup();
			await submitInvalidKeepaliveMax(user);

			const field = screen.getByRole('textbox', {
				name: /^Unanswered keepalives/,
			});
			expect(getTab(/^Connection/)).toHaveAttribute('aria-selected', 'true');
			expect(field).toHaveFocus();
			expect(field).toBeInvalid();
		});

		it('should not save while a field is invalid', async () => {
			const user = userEvent.setup();
			const { gateway } = await submitInvalidKeepaliveMax(user);

			expect(gateway.updateSettings).not.toHaveBeenCalled();
		});

		it('should put the error count in the name of the tab', async () => {
			const user = userEvent.setup();
			await submitInvalidKeepaliveMax(user);

			expect(getTab('Connection 1 error')).toBeInTheDocument();
		});

		it('should count several errors of a tab in the plural', async () => {
			openSettings(buildSettings(), 'connection');
			const user = userEvent.setup();
			await renderDialog();
			await user.clear(
				screen.getByRole('textbox', {
					name: /^Default listen address \(local\)/,
				})
			);
			await user.clear(
				screen.getByRole('textbox', {
					name: /^Default listen address \(remote\)/,
				})
			);

			await clickSave(user);

			expect(getTab('Connection 2 errors')).toBeInTheDocument();
		});

		it('should count the errors of each tab', async () => {
			openSettings(buildSettings(), 'standby');
			const user = userEvent.setup();
			await renderDialog();
			const delay = screen.getByRole('textbox', {
				name: /^Return to standby after/,
			});
			await user.clear(delay);
			await user.type(delay, '0');

			await clickSave(user);

			expect(getTab('Standby 1 error')).toBeInTheDocument();
			expect(getTab('Connection')).toBeInTheDocument();
		});

		it('should drop the badge on the next submit once the error is fixed', async () => {
			const user = userEvent.setup();
			await submitInvalidKeepaliveMax(user);
			const keepaliveMax = screen.getByRole('textbox', {
				name: /^Unanswered keepalives/,
			});
			await user.clear(keepaliveMax);
			await user.type(keepaliveMax, '3');

			await clickSave(user);

			await waitFor(() => expect(getTab('Connection')).toBeInTheDocument());
		});
	});

	it('should show the daemon validation messages in a focused alert and stay open', async () => {
		openSettings();
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.updateSettings.mockRejectedValueOnce(
			new DaemonCommandError('validation_failed', 'invalid', [
				'Invalid address.',
			])
		);
		await renderDialog(gateway);
		await selectTab(user, 'Secrets');

		await clickSave(user);

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent('Invalid address.');
		expect(alert).toHaveFocus();
		expect(
			screen.getByRole('dialog', { name: 'Settings' })
		).toBeInTheDocument();
		expect(useUiStore.getState().panel.kind).toBe('settings');
	});

	describe('applications', () => {
		const mine = buildOpener({
			id: 'dbeaver',
			name: 'DBeaver',
			command: 'dbeaver',
			args: ['-con', 'host={host}'],
			ports: [5432],
			source: 'user',
		});

		async function addViewer(user: User) {
			await user.click(
				screen.getByRole('button', { name: 'Add an application' })
			);
			await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'Viewer');
			await user.type(
				screen.getByRole('textbox', { name: /^Command/ }),
				'viewer'
			);
			await user.click(screen.getByRole('button', { name: 'Add' }));
		}

		it('should only change the draft when applications are added, edited and removed', async () => {
			useDaemonStore.setState({ openers: [mine] });
			openSettings(buildSettings(), 'opening');
			const user = userEvent.setup();
			const { gateway } = await renderDialog();

			await addViewer(user);
			await user.click(screen.getByRole('button', { name: 'Edit DBeaver' }));
			await user.click(screen.getByRole('button', { name: 'Update' }));
			await user.click(screen.getByRole('button', { name: 'Delete DBeaver' }));

			expect(gateway.updateSettings).not.toHaveBeenCalled();
			expect(useDaemonStore.getState().openers).toEqual([mine]);
		});

		it('should send the settings and the applications in one call and store the returned openers', async () => {
			useDaemonStore.setState({ openers: [mine] });
			const settings = buildSettings();
			openSettings(settings, 'opening');
			const user = userEvent.setup();
			const gateway = createFakeGateway();
			const returned = [buildOpener({ id: 'viewer', source: 'user' })];
			gateway.updateSettings.mockResolvedValue(returned);
			await renderDialog(gateway);
			await addViewer(user);

			await clickSave(user);

			await waitFor(() =>
				expect(gateway.updateSettings).toHaveBeenCalledTimes(1)
			);
			expect(gateway.updateSettings).toHaveBeenCalledWith(settings, [
				buildOpenerSpec(),
				{
					id: 'viewer',
					name: 'Viewer',
					command: 'viewer',
					args: [],
					ports: [],
				},
			]);
			await waitFor(() =>
				expect(useDaemonStore.getState().openers).toEqual(returned)
			);
		});

		it('should block the save while the application form has input', async () => {
			openSettings(buildSettings(), 'opening');
			const user = userEvent.setup();
			const { gateway } = await renderDialog();
			await user.click(
				screen.getByRole('button', { name: 'Add an application' })
			);
			await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'Viewer');
			await selectTab(user, 'Secrets');

			await clickSave(user);

			expect(gateway.updateSettings).not.toHaveBeenCalled();
			expect(await screen.findByRole('alert')).toHaveTextContent(
				'An application form still has input'
			);
			expect(getTab('Opening')).toHaveAttribute('aria-selected', 'true');
			await waitFor(() =>
				expect(screen.getByRole('textbox', { name: /^Name/ })).toHaveFocus()
			);
		});

		it('should not block the save for an application form left untouched', async () => {
			openSettings(buildSettings(), 'opening');
			const user = userEvent.setup();
			const { gateway } = await renderDialog();
			await user.click(
				screen.getByRole('button', { name: 'Add an application' })
			);

			await clickSave(user);

			await waitFor(() => expect(gateway.updateSettings).toHaveBeenCalled());
		});
	});

	describe('forgetting remembered passphrases', () => {
		const openSecrets = async (gateway = createFakeGateway()) => {
			openSettings(buildSettings(), 'secrets');
			return renderDialog(gateway);
		};
		const clickForget = (user: User) =>
			user.click(
				screen.getByRole('button', { name: 'Forget remembered passphrases' })
			);

		it('should ask for confirmation before forgetting', async () => {
			const user = userEvent.setup();
			const { gateway } = await openSecrets();

			await clickForget(user);

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Forget remembered passphrases?',
				})
			).toBeInTheDocument();
			expect(gateway.forgetSecrets).not.toHaveBeenCalled();
		});

		it('should not forget when cancelled', async () => {
			const user = userEvent.setup();
			const { gateway } = await openSecrets();
			await clickForget(user);

			const dialog = await screen.findByRole('alertdialog');
			await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));

			expect(gateway.forgetSecrets).not.toHaveBeenCalled();
		});

		it('should forget and toast the count when confirmed', async () => {
			const user = userEvent.setup();
			const gateway = createFakeGateway();
			gateway.forgetSecrets.mockResolvedValue({ forgottenCount: 3 });
			await openSecrets(gateway);
			await clickForget(user);

			await user.click(await screen.findByRole('button', { name: 'Forget' }));

			await waitFor(() =>
				expect(useToastStore.getState().toasts[0]?.message).toBe(
					'3 passphrases forgotten'
				)
			);
		});

		it('should toast an error when forgetting fails', async () => {
			const user = userEvent.setup();
			const gateway = createFakeGateway();
			gateway.forgetSecrets.mockRejectedValue(
				new DaemonCommandError('internal', 'keyring locked')
			);
			await openSecrets(gateway);
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

	describe('cancelling', () => {
		const clickCancel = (user: User) =>
			user.click(screen.getByRole('button', { name: 'Cancel' }));

		async function changeLanguage(user: User) {
			await user.selectOptions(
				screen.getByRole('combobox', { name: 'Display language' }),
				'Français'
			);
		}

		it('should close at once without changes', async () => {
			openSettings();
			const user = userEvent.setup();
			await renderDialog();

			await clickCancel(user);

			await waitFor(() =>
				expect(useUiStore.getState().panel.kind).toBe('none')
			);
			expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
		});

		it('should ask to confirm when a setting changed', async () => {
			openSettings();
			const user = userEvent.setup();
			await renderDialog();
			await changeLanguage(user);

			await clickCancel(user);

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Discard your changes?',
				})
			).toBeInTheDocument();
			expect(useUiStore.getState().panel.kind).toBe('settings');
		});

		it('should keep the dialog and the changes on Keep editing', async () => {
			openSettings();
			const user = userEvent.setup();
			await renderDialog();
			await changeLanguage(user);
			await clickCancel(user);

			await user.click(
				await screen.findByRole('button', { name: 'Keep editing' })
			);

			await waitFor(() =>
				expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
			);
			expect(useUiStore.getState().panel.kind).toBe('settings');
			expect(
				screen.getByRole('combobox', { name: 'Display language' })
			).toHaveValue('fr');
		});

		it('should close on Discard', async () => {
			openSettings();
			const user = userEvent.setup();
			const { gateway } = await renderDialog();
			await changeLanguage(user);
			await clickCancel(user);

			await user.click(await screen.findByRole('button', { name: 'Discard' }));

			await waitFor(() =>
				expect(useUiStore.getState().panel.kind).toBe('none')
			);
			expect(gateway.updateSettings).not.toHaveBeenCalled();
		});

		it.each([
			['Escape', () => userEvent.keyboard('{Escape}')],
			[
				'the close button',
				() => userEvent.click(screen.getByRole('button', { name: 'Close' })),
			],
		])(
			'should ask to confirm when dismissed with %s and a setting changed',
			async (_label, dismiss) => {
				openSettings();
				const user = userEvent.setup();
				await renderDialog();
				await changeLanguage(user);

				await dismiss();

				expect(
					await screen.findByRole('alertdialog', {
						name: 'Discard your changes?',
					})
				).toBeInTheDocument();
				expect(useUiStore.getState().panel.kind).toBe('settings');
			}
		);

		it('should keep the dialog, the changes and the focus when Keep editing follows Escape', async () => {
			openSettings();
			const user = userEvent.setup();
			await renderDialog();
			await changeLanguage(user);
			await user.keyboard('{Escape}');

			await user.click(
				await screen.findByRole('button', { name: 'Keep editing' })
			);

			await waitFor(() =>
				expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
			);
			expect(useUiStore.getState().panel.kind).toBe('settings');
			const languageSelect = screen.getByRole('combobox', {
				name: 'Display language',
			});
			expect(languageSelect).toHaveValue('fr');
			expect(languageSelect).toHaveFocus();
		});

		it('should close on Discard after Escape', async () => {
			openSettings();
			const user = userEvent.setup();
			const { gateway } = await renderDialog();
			await changeLanguage(user);
			await user.keyboard('{Escape}');

			await user.click(await screen.findByRole('button', { name: 'Discard' }));

			await waitFor(() =>
				expect(useUiStore.getState().panel.kind).toBe('none')
			);
			expect(gateway.updateSettings).not.toHaveBeenCalled();
		});

		it('should close at once with Escape and no changes', async () => {
			openSettings();
			const user = userEvent.setup();
			await renderDialog();

			await user.keyboard('{Escape}');

			await waitFor(() =>
				expect(useUiStore.getState().panel.kind).toBe('none')
			);
			expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
		});

		it('should ask to confirm when only an application was added', async () => {
			openSettings(buildSettings(), 'opening');
			const user = userEvent.setup();
			await renderDialog();
			await user.click(
				screen.getByRole('button', { name: 'Add an application' })
			);
			await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'Viewer');
			await user.type(
				screen.getByRole('textbox', { name: /^Command/ }),
				'viewer'
			);
			await user.click(screen.getByRole('button', { name: 'Add' }));

			await clickCancel(user);

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Discard your changes?',
				})
			).toBeInTheDocument();
		});
	});
});
