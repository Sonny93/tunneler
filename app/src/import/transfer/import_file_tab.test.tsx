import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { ConfigImportPlanDto } from '#/daemon/protocol';
import { ImportFileTab } from '#/import/transfer/import_file_tab';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useDaemonStore } from '#/state/daemon_store';
import {
	buildConfigImportEntry,
	buildConfigImportPlan,
	buildTunnel,
} from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { findFilledLiveRegion } from '#/testing/filled_live_region';
import { renderWithGateway } from '#/testing/render_with_gateway';

const FILE = { fileName: 'tunnels.toml', contents: 'CONTENTS' };
const CHOOSE_FILE = { name: 'Choose a file…' };

const buildTunnelSpec = (listenPort: number, tags: string[] = []) => ({
	host: 'nova',
	kind: 'local' as const,
	listenPort,
	tags,
});

const buildEveryCasePlan = (): ConfigImportPlanDto =>
	buildConfigImportPlan({
		unknownHosts: ['oldbox'],
		entries: [
			buildConfigImportEntry({
				index: 0,
				tunnel: buildTunnelSpec(3333, ['dev']),
				fileHost: 'oldbox',
				warnings: [
					{ kind: 'unknownHost', host: 'oldbox' },
					{ kind: 'openerDropped', opener: 'gone' },
				],
			}),
			buildConfigImportEntry({
				index: 1,
				tunnel: buildTunnelSpec(3334),
				risks: [
					{ kind: 'nonLoopbackListen', address: '0.0.0.0' },
					{ kind: 'remoteNonLoopbackTarget', targetHost: '10.0.0.5' },
					{ kind: 'startsOnLaunch', tags: ['boot'] },
				],
				selectedByDefault: false,
			}),
			buildConfigImportEntry({
				index: 2,
				tunnel: buildTunnelSpec(3335),
				status: {
					kind: 'alreadyConfigured',
					existingId: 'saved',
					ignoredTags: ['dev'],
				},
				selectedByDefault: false,
			}),
			buildConfigImportEntry({
				index: 3,
				tunnel: buildTunnelSpec(3336),
				status: { kind: 'listenConflict', holderId: 'saved' },
				selectedByDefault: false,
			}),
			buildConfigImportEntry({
				index: 4,
				tunnel: buildTunnelSpec(3337),
				status: { kind: 'duplicateInFile', earlierIndex: 0 },
				selectedByDefault: false,
			}),
			buildConfigImportEntry({
				index: 5,
				tunnel: buildTunnelSpec(3338),
				status: { kind: 'invalid', messages: ['port 0 is not valid'] },
				selectedByDefault: false,
			}),
		],
	});

function renderTab(plan: ConfigImportPlanDto = buildEveryCasePlan()) {
	const gateway = createFakeGateway();
	gateway.pickImportFile.mockResolvedValue(FILE);
	gateway.previewConfigImport.mockResolvedValue(plan);
	useDaemonStore.setState({
		hosts: ['nova', 'other'],
		tunnels: [
			buildTunnel({ id: 'saved', host: 'nova', tags: ['dev'] }),
			buildTunnel({ id: 'mail', host: 'vega', tags: ['mail'], listenPort: 25 }),
		],
	});
	return renderWithGateway(<ImportFileTab />, gateway);
}

async function chooseFile(user: ReturnType<typeof userEvent.setup>) {
	await user.click(screen.getByRole('button', CHOOSE_FILE));
	await screen.findByRole('heading', { name: 'Preview of tunnels.toml' });
}

describe('ImportFileTab', () => {
	describe('export', () => {
		it('should export every tunnel without a filter and save it with the native dialog', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.exportConfig.mockResolvedValue({
				contents: 'EXPORT',
				tunnelCount: 2,
			});

			await user.click(screen.getByRole('button', { name: 'Export…' }));

			await waitFor(() =>
				expect(gateway.exportToFile).toHaveBeenCalledWith(
					'EXPORT',
					'tunneler-export.toml'
				)
			);
			expect(gateway.exportConfig).toHaveBeenCalledWith([], []);
			expect(await screen.findByText('2 tunnels exported')).toBeInTheDocument();
			expect(useAnnouncerStore.getState().polite.message).toBe(
				'2 tunnels exported'
			);
		});

		it('should send the checked hosts and tags as filters', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.exportConfig.mockResolvedValue({
				contents: 'EXPORT',
				tunnelCount: 1,
			});

			await user.click(screen.getByRole('checkbox', { name: 'vega' }));
			await user.click(screen.getByRole('checkbox', { name: 'dev' }));
			await user.click(screen.getByRole('button', { name: 'Export…' }));

			await waitFor(() =>
				expect(gateway.exportConfig).toHaveBeenCalledWith(['vega'], ['dev'])
			);
		});

		it('should stay silent and give the focus back to the button when the dialog is cancelled', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.exportConfig.mockResolvedValue({
				contents: 'EXPORT',
				tunnelCount: 1,
			});
			gateway.exportToFile.mockResolvedValue({ saved: false });

			await user.click(screen.getByRole('button', { name: 'Export…' }));

			await waitFor(() => expect(gateway.exportToFile).toHaveBeenCalled());
			await waitFor(() =>
				expect(screen.getByRole('button', { name: 'Export…' })).toHaveFocus()
			);
			expect(
				screen.queryByText(/^\d+ tunnels? exported$/)
			).not.toBeInTheDocument();
			expect(useAnnouncerStore.getState().polite.message).toBe('');
		});

		it('should not open the dialog and say so when no tunnel matches', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();

			await user.click(screen.getByRole('button', { name: 'Export…' }));

			await waitFor(() =>
				expect(
					screen.getByText('No saved tunnel matches these filters.')
				).toBeInTheDocument()
			);
			expect(gateway.exportToFile).not.toHaveBeenCalled();
		});

		it('should have no axe violation with an error summary', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.exportConfig.mockRejectedValue(
				new DaemonCommandError('disconnected', 'lost')
			);
			await user.click(screen.getByRole('button', { name: 'Export…' }));
			await findFilledLiveRegion('alert');

			expect(await axe(document.body)).toHaveNoViolations();
		});

		it('should show the daemon message of an export that is too large, marked as English', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			const message =
				'the export is larger than 256 KiB; filter by host or tag';
			gateway.exportConfig.mockRejectedValue(
				new DaemonCommandError('validation_failed', 'invalid', [message])
			);

			await user.click(screen.getByRole('button', { name: 'Export…' }));

			const alert = await findFilledLiveRegion('alert');
			expect(within(alert).getByText(message)).toHaveAttribute('lang', 'en');
			expect(await axe(document.body)).toHaveNoViolations();
		});

		it('should have no axe violation with a result line', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.exportConfig.mockResolvedValue({
				contents: 'EXPORT',
				tunnelCount: 2,
			});
			await user.click(screen.getByRole('button', { name: 'Export…' }));
			await screen.findByText('2 tunnels exported');

			expect(await axe(document.body)).toHaveNoViolations();
		});

		it('should report a failure in a focused alert', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.exportConfig.mockRejectedValue(
				new DaemonCommandError('disconnected', 'lost')
			);

			await user.click(screen.getByRole('button', { name: 'Export…' }));

			const alert = await findFilledLiveRegion('alert');
			expect(alert).toHaveTextContent('Export failed:');
			expect(alert).toHaveFocus();
		});
	});

	describe('import', () => {
		it('should do nothing visible when the file dialog is cancelled', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.pickImportFile.mockResolvedValue(null);

			await user.click(screen.getByRole('button', CHOOSE_FILE));

			await waitFor(() =>
				expect(screen.getByRole('button', CHOOSE_FILE)).toHaveFocus()
			);
			expect(gateway.previewConfigImport).not.toHaveBeenCalled();
		});

		it('should preview the file, announce the count and focus the preview heading', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();

			await chooseFile(user);

			expect(gateway.previewConfigImport).toHaveBeenCalledWith('CONTENTS', {});
			expect(
				screen.getByRole('heading', { name: 'Preview of tunnels.toml' })
			).toHaveFocus();
			expect(useAnnouncerStore.getState().polite.message).toBe(
				'6 tunnels detected'
			);
		});

		it('should check the safe entry and leave the risky one unchecked with its reasons as text', async () => {
			const user = userEvent.setup();
			renderTab();

			await chooseFile(user);

			const safe = screen.getByRole('checkbox', { name: '3333 to nova:3333' });
			const risky = screen.getByRole('checkbox', { name: '3334 to nova:3334' });
			expect(safe).toBeChecked();
			expect(risky).not.toBeChecked();
			expect(risky).toHaveAccessibleDescription(
				/Risk: listens on 0\.0\.0\.0, reachable from other machines/
			);
			expect(risky).toHaveAccessibleDescription(
				/Risk: forwards remote connections to 10\.0\.0\.5/
			);
			expect(risky).toHaveAccessibleDescription(
				/Risk: its tags start on launch \(boot\)/
			);
			expect(
				screen.getByRole('button', { name: 'Import 1 tunnel' })
			).toBeInTheDocument();
		});

		it('should describe the tags and warnings of an entry', async () => {
			const user = userEvent.setup();
			renderTab();

			await chooseFile(user);

			expect(
				screen.getByRole('checkbox', { name: '3333 to nova:3333' })
			).toHaveAccessibleDescription(
				'tags: dev · Will be added · No ssh_config Host entry names oldbox exactly · Opener gone is not available here and was removed'
			);
		});

		it.each([
			['3335 to nova:3335', 'Already configured (tags not applied: dev)'],
			['3336 to nova:3336', 'Port already used by 3333 → nova:3333'],
			[
				'3337 to nova:3337',
				'Same route as tunnel 1 of the file (3333 to nova:3333)',
			],
		])(
			'should show %s as text without a checkbox: %s',
			async (label, reason) => {
				const user = userEvent.setup();
				renderTab();

				await chooseFile(user);

				expect(
					screen.queryByRole('checkbox', { name: label })
				).not.toBeInTheDocument();
				const fieldset = screen.getByRole('group', {
					name: 'Tunnels in the file (6)',
				});
				expect(within(fieldset).getByText(label)).toBeInTheDocument();
				expect(within(fieldset).getByText(reason)).toBeInTheDocument();
			}
		);

		it('should show the English messages of an invalid entry apart, marked as English', async () => {
			const user = userEvent.setup();
			renderTab();

			await chooseFile(user);

			const message = screen.getByText('port 0 is not valid');
			expect(message).toHaveAttribute('lang', 'en');
			expect(message.parentElement).toHaveTextContent(
				'Invalid: port 0 is not valid'
			);
		});

		it('should number the entries from 1 like the reasons that point to them', async () => {
			const user = userEvent.setup();
			renderTab();

			await chooseFile(user);

			const group = screen.getByRole('group', {
				name: 'Tunnels in the file (6)',
			});
			const items = within(group).getAllByRole('listitem');
			expect(items.map((item) => item.getAttribute('value'))).toEqual([
				'1',
				'2',
				'3',
				'4',
				'5',
				'6',
			]);
		});

		it('should say a risky entry can be added but is not selected by default', async () => {
			const user = userEvent.setup();
			renderTab();

			await chooseFile(user);

			expect(
				screen.getByRole('checkbox', { name: '3334 to nova:3334' })
			).toHaveAccessibleDescription(/Can be added, not selected by default/);
		});

		it('should word the listen risk for the server when the tunnel is remote', async () => {
			const user = userEvent.setup();
			renderTab(
				buildConfigImportPlan({
					entries: [
						buildConfigImportEntry({
							tunnel: { ...buildTunnelSpec(8080), kind: 'remote' },
							risks: [{ kind: 'nonLoopbackListen', address: '0.0.0.0' }],
							selectedByDefault: false,
						}),
					],
				})
			);

			await chooseFile(user);

			expect(
				screen.getByRole('checkbox', { name: /port 8080 of server nova/ })
			).toHaveAccessibleDescription(
				/Risk: listens on 0\.0\.0\.0 on the server, reachable from the server's network/
			);
		});

		it('should announce how many entries are selected, risky ones included, by Select all and Deselect all', async () => {
			const user = userEvent.setup();
			renderTab();
			await chooseFile(user);

			await user.click(screen.getByRole('button', { name: 'Select all' }));
			expect(useAnnouncerStore.getState().polite.message).toBe(
				'2 tunnels selected, including 1 risky'
			);

			await user.click(screen.getByRole('button', { name: 'Deselect all' }));
			expect(useAnnouncerStore.getState().polite.message).toBe(
				'No tunnel selected'
			);
		});

		it('should keep the empty import button focusable and say why', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			await chooseFile(user);
			await user.click(screen.getByRole('button', { name: 'Deselect all' }));
			const button = screen.getByRole('button', { name: 'Import 0 tunnels' });

			button.focus();
			await user.click(button);

			expect(button).toHaveFocus();
			expect(button).toHaveAccessibleDescription('Select at least one tunnel');
			expect(gateway.applyConfigImport).not.toHaveBeenCalled();
		});

		it('should describe the unknown host only while it is not mapped', async () => {
			const user = userEvent.setup();
			renderTab();
			await chooseFile(user);
			const select = screen.getByRole('combobox', {
				name: 'Use this alias for oldbox',
			});
			expect(select).toHaveAccessibleDescription(
				'No ssh_config Host entry names it exactly.'
			);

			await user.selectOptions(select, 'other');

			await waitFor(() =>
				expect(select).not.toHaveAccessibleDescription(/ssh_config/)
			);
		});

		it('should say the selection was reset when the user had changed it before a new preview', async () => {
			const user = userEvent.setup();
			renderTab();
			await chooseFile(user);
			await user.click(
				screen.getByRole('checkbox', { name: '3334 to nova:3334' })
			);

			await user.selectOptions(
				screen.getByRole('combobox', { name: 'Use this alias for oldbox' }),
				'other'
			);

			await waitFor(() =>
				expect(useAnnouncerStore.getState().polite.message).toBe(
					'Preview updated, selection reset, 1 tunnel selected'
				)
			);
		});

		it('should apply the second of two quick alias changes and never revert the choice', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			await chooseFile(user);
			const answers: ((plan: ConfigImportPlanDto) => void)[] = [];
			gateway.previewConfigImport.mockImplementation(
				() =>
					new Promise((resolve) => {
						answers.push(resolve);
					})
			);
			const select = screen.getByRole('combobox', {
				name: 'Use this alias for oldbox',
			});

			await user.selectOptions(select, 'other');
			await user.selectOptions(select, 'nova');
			expect(select).toHaveValue('nova');
			expect(gateway.previewConfigImport).toHaveBeenCalledTimes(2);
			expect(
				screen.getByRole('button', { name: /^Import \d+ tunnel/ })
			).toHaveAccessibleDescription('Updating the preview…');
			useAnnouncerStore.setState({ polite: { id: 0, message: '' } });

			await act(async () => {
				answers[0]?.(buildEveryCasePlan());
				await Promise.resolve();
			});
			await waitFor(() => expect(answers).toHaveLength(2));
			expect(gateway.previewConfigImport).toHaveBeenLastCalledWith('CONTENTS', {
				oldbox: 'nova',
			});
			expect(select).toHaveValue('nova');
			expect(useAnnouncerStore.getState().polite.message).toBe('');

			await act(async () => {
				answers[1]?.(buildEveryCasePlan());
				await Promise.resolve();
			});

			await waitFor(() =>
				expect(useAnnouncerStore.getState().polite.message).toBe(
					'Preview updated, 1 tunnel selected'
				)
			);
			expect(select).toHaveValue('nova');
		});

		it('should have no axe violation while the preview is being updated', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			await chooseFile(user);
			gateway.previewConfigImport.mockImplementation(
				() => new Promise(() => undefined)
			);
			await user.selectOptions(
				screen.getByRole('combobox', { name: 'Use this alias for oldbox' }),
				'other'
			);

			expect(await axe(document.body)).toHaveNoViolations();
		});

		it('should expose the import button as aria-disabled while it is applying', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.applyConfigImport.mockImplementation(
				() => new Promise(() => undefined)
			);
			await chooseFile(user);

			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));

			const button = screen.getByRole('button', { name: /^Import 1 tunnel/ });
			expect(button).toHaveAttribute('aria-disabled', 'true');
			expect(button).not.toHaveAccessibleDescription();
		});

		it('should have no axe violation while the import is being applied', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.applyConfigImport.mockImplementation(
				() => new Promise(() => undefined)
			);
			await chooseFile(user);
			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));

			expect(await axe(document.body)).toHaveNoViolations();
		});

		it('should let the user check a risky entry on purpose', async () => {
			const user = userEvent.setup();
			renderTab();
			await chooseFile(user);

			await user.click(
				screen.getByRole('checkbox', { name: '3334 to nova:3334' })
			);

			expect(
				screen.getByRole('button', { name: 'Import 2 tunnels' })
			).toBeInTheDocument();
		});

		it('should select every importable entry and none', async () => {
			const user = userEvent.setup();
			renderTab();
			await chooseFile(user);

			await user.click(screen.getByRole('button', { name: 'Select all' }));
			expect(
				screen.getByRole('button', { name: 'Import 2 tunnels' })
			).toBeEnabled();

			await user.click(screen.getByRole('button', { name: 'Deselect all' }));
			expect(
				screen.getByRole('button', { name: 'Import 0 tunnels' })
			).toHaveAttribute('aria-disabled', 'true');
		});

		it('should offer an alias for an unknown host and preview again with the host map', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			await chooseFile(user);
			gateway.previewConfigImport.mockResolvedValue(
				buildConfigImportPlan({
					entries: [
						buildConfigImportEntry({ fileHost: 'oldbox' }),
						buildConfigImportEntry({
							index: 1,
							tunnel: buildTunnelSpec(3334),
						}),
					],
				})
			);

			const select = screen.getByRole('combobox', {
				name: 'Use this alias for oldbox',
			});
			expect(
				within(select).getByRole('option', { name: 'Keep oldbox' })
			).toBeInTheDocument();
			await user.selectOptions(select, 'other');

			await waitFor(() =>
				expect(gateway.previewConfigImport).toHaveBeenLastCalledWith(
					'CONTENTS',
					{ oldbox: 'other' }
				)
			);
			await waitFor(() =>
				expect(
					screen.getByRole('button', { name: 'Import 2 tunnels' })
				).toBeInTheDocument()
			);
			expect(
				screen.getByRole('combobox', { name: 'Use this alias for oldbox' })
			).toHaveValue('other');
			expect(useAnnouncerStore.getState().polite.message).toBe(
				'Preview updated, 2 tunnels selected'
			);
		});

		it('should apply the checked entries with the host map and announce the result', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.applyConfigImport.mockResolvedValue({
				added: [{ index: 0, id: 'new' }],
				skipped: [
					{ index: 3, status: { kind: 'listenConflict', holderId: 'saved' } },
					{ index: 5, status: { kind: 'invalid', messages: ['bad port'] } },
				],
			});
			await chooseFile(user);
			await user.selectOptions(
				screen.getByRole('combobox', { name: 'Use this alias for oldbox' }),
				'other'
			);
			await waitFor(() =>
				expect(gateway.previewConfigImport).toHaveBeenCalledTimes(2)
			);

			await user.click(
				screen.getByRole('button', { name: /^Import \d+ tunnel/ })
			);

			await waitFor(() =>
				expect(gateway.applyConfigImport).toHaveBeenCalledWith(
					'CONTENTS',
					{ oldbox: 'other' },
					[0]
				)
			);
			const heading = await screen.findByRole('heading', {
				name: 'Import of tunnels.toml',
			});
			expect(heading).toHaveFocus();
			expect(screen.getByText('1 tunnel imported')).toBeInTheDocument();
			expect(
				screen.getByText(
					'3336 to nova:3336 skipped: Port already used by 3333 → nova:3333'
				)
			).toBeInTheDocument();
			expect(screen.getByText('bad port')).toHaveAttribute('lang', 'en');
			expect(useAnnouncerStore.getState().polite.message).toBe(
				'1 tunnel imported'
			);
			expect(
				screen.queryByRole('heading', { name: 'Preview of tunnels.toml' })
			).not.toBeInTheDocument();
		});

		it.each([
			[
				{ reason: 'tooLarge', limit: 262_144 },
				'The file is larger than 256 KiB.',
			],
			[
				{ reason: 'unsafeCharacters', entry: 1, field: 'host' },
				'Tunnel 2 of the file has control or hidden characters in the field host, so the whole file was refused.',
			],
			[
				{ reason: 'notTunnelerFile' },
				'This is not a Tunneler export or tunnels file.',
			],
		])(
			'should explain an invalid file (%j) from the picker',
			async (data, expected) => {
				const user = userEvent.setup();
				const { gateway } = renderTab();
				gateway.pickImportFile.mockRejectedValue(
					new DaemonCommandError('invalid_import_file', 'raw', data)
				);

				await user.click(screen.getByRole('button', CHOOSE_FILE));

				const alert = await findFilledLiveRegion('alert');
				expect(alert).toHaveTextContent('Import failed:');
				expect(alert).toHaveTextContent(expected);
				expect(alert).toHaveFocus();
			}
		);

		it('should mark the parser message of a syntax error as English', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.previewConfigImport.mockRejectedValue(
				new DaemonCommandError('invalid_import_file', 'raw', {
					reason: 'syntax',
					line: 3,
					message: 'bad key',
				})
			);
			await user.click(screen.getByRole('button', CHOOSE_FILE));

			const alert = await findFilledLiveRegion('alert');

			expect(within(alert).getByText('bad key')).toHaveAttribute('lang', 'en');
		});

		it('should explain an invalid file refused by the daemon and drop the previous preview', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			await chooseFile(user);
			gateway.previewConfigImport.mockRejectedValue(
				new DaemonCommandError('invalid_import_file', 'raw', {
					reason: 'syntax',
					line: 3,
					message: 'bad key',
				})
			);

			await user.click(screen.getByRole('button', CHOOSE_FILE));

			const alert = await findFilledLiveRegion('alert');
			expect(alert).toHaveTextContent(
				'The file cannot be read (line 3): bad key'
			);
			expect(
				screen.queryByRole('heading', { name: 'Preview of tunnels.toml' })
			).not.toBeInTheDocument();
		});
	});

	describe('axe', () => {
		it('should have no violation when empty', async () => {
			renderTab();

			expect(await axe(document.body)).toHaveNoViolations();
		});

		it('should have no violation with a preview holding every status, risk and warning', async () => {
			const user = userEvent.setup();
			renderTab();
			await chooseFile(user);

			expect(await axe(document.body)).toHaveNoViolations();
		});

		it('should have no violation after a result', async () => {
			const user = userEvent.setup();
			const { gateway } = renderTab();
			gateway.applyConfigImport.mockResolvedValue({
				added: [{ index: 0, id: 'new' }],
				skipped: [
					{ index: 3, status: { kind: 'duplicateInFile', earlierIndex: 0 } },
				],
			});
			await chooseFile(user);
			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));
			await screen.findByRole('heading', { name: 'Import of tunnels.toml' });

			expect(await axe(document.body)).toHaveNoViolations();
		});
	});
});
