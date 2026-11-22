import { i18n } from '@lingui/core';
import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { ImportConflictDto, ImportPreviewDto } from '#/daemon/protocol';
import { activateLocale } from '#/i18n/activate_locale';
import { ImportDrawer } from '#/import/import_drawer';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { DIGITS_TAG_MESSAGE } from '#/shared/reserved_tag_problem';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { classifyAgainstStore } from '#/testing/classify_against_store';
import {
	buildConfigImportEntry,
	buildConfigImportPlan,
	buildImportCandidate,
	buildImportPreview,
	buildSettings,
	buildTunnel,
} from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { findFilledLiveRegion } from '#/testing/filled_live_region';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { getShownToasts } from '#/testing/shown_toasts';

const POWERSHELL_PORTS = [3333, 3334, 3335, 3336, 3337, 3338, 3339, 3340];
const SCRIPT = 'ssh -L 3333:localhost:3333 nova';

const buildPowerShellPreview = (): ImportPreviewDto =>
	buildImportPreview({
		candidates: POWERSHELL_PORTS.map((listenPort, index) =>
			buildImportCandidate({ listenPort, line: index + 1 })
		),
	});

function renderDrawer(preview: ImportPreviewDto = buildPowerShellPreview()) {
	const gateway = createFakeGateway();
	gateway.classifyImports.mockImplementation(classifyAgainstStore);
	gateway.importScript.mockResolvedValue(preview);
	gateway.importSshConfig.mockResolvedValue(preview);
	useDaemonStore.setState({
		settings: buildSettings(),
		hosts: ['nova', 'other'],
	});
	useUiStore.getState().openImport();
	return renderWithGateway(<ImportDrawer />, gateway);
}

async function analyzeScript(user: ReturnType<typeof userEvent.setup>) {
	await user.type(
		screen.getByRole('textbox', {
			name: /Script, list of ports or -L\/-R\/-D options/,
		}),
		SCRIPT
	);
	await user.click(screen.getByRole('button', { name: 'Analyze' }));
	await screen.findByRole('heading', { name: 'Analysis result' });
}

describe('ImportDrawer', () => {
	it('should render nothing while another panel is open', () => {
		useUiStore.getState().openSettings();
		renderWithGateway(<ImportDrawer />);

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('should offer the two sources as tabs', () => {
		renderDrawer();

		expect(
			screen.getByRole('dialog', { name: 'Import and export' })
		).toBeInTheDocument();
		expect(
			screen.getByRole('tab', { name: 'Paste a script' })
		).toBeInTheDocument();
		expect(
			screen.getByRole('tab', { name: 'From ~/.ssh/config' })
		).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'File' })).toBeInTheDocument();
	});

	it('should describe the accepted input', () => {
		renderDrawer();

		expect(
			screen.getByRole('textbox', {
				name: /Script, list of ports or -L\/-R\/-D options/,
			})
		).toHaveAccessibleDescription(/list of ports \(3333, 5432\)/);
	});

	it('should link an error to the empty script field', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Analyze' }));

		const field = screen.getByRole('textbox', {
			name: /Script, list of ports or -L\/-R\/-D options/,
		});
		expect(field).toBeInvalid();
		expect(field).toHaveAccessibleDescription(/Paste a script/);
		expect(gateway.importScript).not.toHaveBeenCalled();
	});

	it('should analyze the script, announce the detected tunnels and focus the preview heading', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await analyzeScript(user);

		expect(gateway.importScript).toHaveBeenCalledWith(SCRIPT);
		expect(screen.getByText('8 tunnels detected')).toBeInTheDocument();
		expect(useAnnouncerStore.getState().polite.message).toBe(
			'8 tunnels detected'
		);
		expect(
			screen.getByRole('heading', { name: 'Analysis result' })
		).toHaveFocus();
	});

	it('should read ~/.ssh/config from its tab', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('tab', { name: 'From ~/.ssh/config' }));
		await user.click(
			screen.getByRole('button', { name: 'Read ~/.ssh/config' })
		);

		expect(
			await screen.findByRole('group', { name: 'Detected tunnels (8)' })
		).toBeInTheDocument();
		expect(gateway.importSshConfig).toHaveBeenCalled();
	});

	it('should report an analysis failure in a focused alert', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.importScript.mockRejectedValue(
			new DaemonCommandError('invalid_input', 'script trop long')
		);

		await user.type(screen.getByRole('textbox'), SCRIPT);
		await user.click(screen.getByRole('button', { name: 'Analyze' }));

		const alert = await findFilledLiveRegion('alert');
		expect(alert).toHaveTextContent('Analysis failed:');
		expect(alert).toHaveTextContent('script trop long');
		expect(alert).toHaveFocus();
	});

	it('should check every candidate, label them with their endpoint and not ask for a host when every candidate has one', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await analyzeScript(user);

		expect(screen.getAllByRole('checkbox')).toHaveLength(8);
		expect(
			screen.getByRole('checkbox', { name: '3333 to nova:3333' })
		).toBeChecked();
		expect(
			screen.getByRole('button', { name: 'Import 8 tunnels' })
		).toBeEnabled();
		expect(
			screen.queryByRole('combobox', { name: /Host of the tunnels/ })
		).not.toBeInTheDocument();
	});

	it('should describe every warning in French with the raw text apart', async () => {
		const user = userEvent.setup();
		renderDrawer(
			buildImportPreview({
				candidates: [buildImportCandidate()],
				warnings: [
					{ kind: 'malformedForward', line: 3, text: '-L nope' },
					{
						kind: 'unsupportedForward',
						line: 5,
						host: 'nova',
						text: '-L /tmp/s:x:1',
					},
					{ kind: 'unknownDestination', destination: 'user@198.51.100.10' },
					{
						kind: 'hostResolutionFailed',
						host: 'old',
						message: 'bad config',
					},
				],
			})
		);

		await analyzeScript(user);

		const section = screen.getByRole('region', { name: 'Warnings' });
		expect(section).toHaveTextContent(
			'Line 3: option ignored, unreadable: -L nope'
		);
		expect(section).toHaveTextContent(
			'Line 5: nova: unsupported forward (Unix socket or remote SOCKS): -L /tmp/s:x:1'
		);
		expect(section).toHaveTextContent(
			'No ssh_config alias matches user@198.51.100.10: choose the host below.'
		);
		expect(section).toHaveTextContent("Couldn't read old: bad config");
		expect(within(section).getByText('bad config')).toHaveAttribute(
			'lang',
			'en'
		);
	});

	it('should uncheck a candidate in conflict and say why', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({ tunnels: [buildTunnel()] });
		renderDrawer();

		await analyzeScript(user);

		const conflicting = screen.getByRole('checkbox', {
			name: '3333 to nova:3333',
		});
		expect(
			await screen.findByRole('button', { name: 'Import 7 tunnels' })
		).toBeInTheDocument();
		expect(conflicting).not.toBeChecked();
		expect(conflicting).toHaveAccessibleDescription(/Already configured/);
	});

	it('should let the user check a candidate in conflict', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({ tunnels: [buildTunnel()] });
		renderDrawer();
		await analyzeScript(user);
		await screen.findByRole('button', { name: 'Import 7 tunnels' });

		await user.click(
			screen.getByRole('checkbox', { name: '3333 to nova:3333' })
		);

		expect(
			screen.getByRole('button', { name: 'Import 8 tunnels' })
		).toBeInTheDocument();
	});

	it('should ask the daemon about the candidates with their host and route when the preview arrives', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer(
			buildImportPreview({
				candidates: [
					buildImportCandidate({
						kind: 'remote',
						listenPort: 8080,
						listenAddress: '0.0.0.0',
						targetHost: '10.0.0.5',
						targetPort: 80,
						comment: 'not sent',
						line: 4,
					}),
				],
			})
		);

		await analyzeScript(user);

		await waitFor(() =>
			expect(gateway.classifyImports).toHaveBeenCalledWith([
				{
					host: 'nova',
					kind: 'remote',
					listenPort: 8080,
					listenAddress: '0.0.0.0',
					targetHost: '10.0.0.5',
					targetPort: 80,
				},
			])
		);
	});

	it('should keep the import button disabled until the daemon answered', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		let answer: (conflicts: (ImportConflictDto | null)[]) => void = () =>
			undefined;
		gateway.classifyImports.mockImplementation(
			() =>
				new Promise((resolve) => {
					answer = resolve;
				})
		);
		await analyzeScript(user);

		const pendingButton = screen.getByRole('button', {
			name: 'Import 8 tunnels',
		});
		expect(pendingButton).toHaveAttribute('aria-disabled', 'true');
		expect(pendingButton).toHaveAccessibleDescription(
			'Checking for conflicts…'
		);
		await user.click(pendingButton);
		expect(gateway.createTunnel).not.toHaveBeenCalled();

		act(() => answer(POWERSHELL_PORTS.map(() => null)));

		await waitFor(() =>
			expect(
				screen.getByRole('button', { name: 'Import 8 tunnels' })
			).toBeEnabled()
		);
	});

	it('should select and unselect everything', async () => {
		const user = userEvent.setup();
		renderDrawer();
		await analyzeScript(user);

		await user.click(screen.getByRole('button', { name: 'Deselect all' }));
		expect(useAnnouncerStore.getState().polite.message).toBe(
			'No tunnel selected'
		);
		const emptyButton = screen.getByRole('button', {
			name: 'Import 0 tunnels',
		});
		expect(emptyButton).toHaveAttribute('aria-disabled', 'true');
		expect(emptyButton).toHaveAccessibleDescription(
			'Select at least one tunnel'
		);

		await user.click(screen.getByRole('button', { name: 'Select all' }));
		expect(useAnnouncerStore.getState().polite.message).toBe(
			'8 tunnels selected'
		);
		expect(
			screen.getByRole('button', { name: 'Import 8 tunnels' })
		).not.toHaveAttribute('aria-disabled');
	});

	it('should keep the empty import button focusable and inert', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		await analyzeScript(user);
		await user.click(screen.getByRole('button', { name: 'Deselect all' }));
		const button = screen.getByRole('button', { name: 'Import 0 tunnels' });

		button.focus();
		await user.click(button);

		expect(button).toHaveFocus();
		expect(gateway.createTunnel).not.toHaveBeenCalled();
	});

	it('should say so and keep importing possible when the check for conflicts fails', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.classifyImports.mockRejectedValue(
			new DaemonCommandError('disconnected', 'lost')
		);

		await analyzeScript(user);

		expect(
			await screen.findByText(
				'Could not check for conflicts; creation will still refuse clashes.'
			)
		).toBeInTheDocument();
		expect(useAnnouncerStore.getState().polite.message).toBe(
			'Could not check for conflicts; creation will still refuse clashes.'
		);
		expect(
			screen.getByRole('button', { name: 'Import 8 tunnels' })
		).not.toHaveAttribute('aria-disabled');
	});

	it('should announce the new number of conflicts after a re-classification', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer(
			buildImportPreview({
				candidates: [
					buildImportCandidate({ host: undefined, listenPort: 5000 }),
				],
			})
		);
		gateway.classifyImports.mockResolvedValue([{ kind: 'alreadyConfigured' }]);
		await analyzeScript(user);
		await user.selectOptions(
			screen.getByRole('combobox', {
				name: /Host of the tunnels without a destination/,
			}),
			'other'
		);

		await waitFor(() =>
			expect(useAnnouncerStore.getState().polite.message).toBe('1 conflict')
		);
	});

	it('should have no axe violations while the daemon has not answered', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.classifyImports.mockImplementation(
			() => new Promise(() => undefined)
		);
		await analyzeScript(user);

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should name the tunnel already listening on the port of a candidate', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({
			tunnels: [buildTunnel({ listenPort: 3334, targetPort: 80 })],
		});
		renderDrawer();

		await analyzeScript(user);

		await waitFor(() =>
			expect(
				screen.getByRole('checkbox', { name: '3334 to nova:3334' })
			).toHaveAccessibleDescription(/Port already used by 3334 → nova:80/)
		);
	});

	describe('candidates without a host', () => {
		const hostlessPreview = () =>
			buildImportPreview({
				candidates: [
					buildImportCandidate({ host: undefined, listenPort: 5000 }),
				],
			});

		it('should classify the candidate again with the chosen default host', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(hostlessPreview());
			await analyzeScript(user);

			await user.selectOptions(
				screen.getByRole('combobox', {
					name: /Host of the tunnels without a destination/,
				}),
				'other'
			);

			await waitFor(() =>
				expect(gateway.classifyImports).toHaveBeenLastCalledWith([
					{ host: 'other', kind: 'local', listenPort: 5000 },
				])
			);
		});

		it('should require a host choice before importing', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(hostlessPreview());
			await analyzeScript(user);

			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));

			const select = screen.getByRole('combobox', {
				name: /Host of the tunnels without a destination/,
			});
			expect(select).toBeRequired();
			expect(select).toBeInvalid();
			expect(select).toHaveFocus();
			expect(gateway.createTunnel).not.toHaveBeenCalled();
		});

		it('should create the tunnel on the chosen host', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(hostlessPreview());
			await analyzeScript(user);

			await user.selectOptions(
				screen.getByRole('combobox', {
					name: /Host of the tunnels without a destination/,
				}),
				'other'
			);
			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));

			await waitFor(() =>
				expect(gateway.createTunnel).toHaveBeenCalledWith({
					host: 'other',
					kind: 'local',
					listenPort: 5000,
					tags: [],
				})
			);
		});
	});

	describe('creation', () => {
		it('should suggest the existing tags and describe the field', async () => {
			const user = userEvent.setup();
			useDaemonStore.setState({
				tunnels: [
					buildTunnel({ id: 'a', tags: ['dev', 'Db'] }),
					buildTunnel({ id: 'b', tags: ['dev'] }),
				],
			});
			renderDrawer();
			await analyzeScript(user);

			const field = screen.getByRole('combobox', { name: 'Tag to add' });

			expect(field).toHaveAccessibleDescription(/Optional/);
			expect(
				Array.from((field as HTMLInputElement).list?.options ?? []).map(
					(option) => option.value
				)
			).toEqual(['Db', 'dev']);
		});

		it('should add nothing when the tag field is empty', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(
				buildImportPreview({ candidates: [buildImportCandidate()] })
			);
			await analyzeScript(user);

			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));

			await waitFor(() =>
				expect(gateway.createTunnel).toHaveBeenCalledWith(
					expect.objectContaining({ tags: [] })
				)
			);
		});

		it('should not duplicate a tag typed twice', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(
				buildImportPreview({ candidates: [buildImportCandidate()] })
			);
			await analyzeScript(user);

			await user.type(
				screen.getByRole('combobox', { name: 'Tag to add' }),
				' dev , dev'
			);
			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));

			await waitFor(() =>
				expect(gateway.createTunnel).toHaveBeenCalledWith(
					expect.objectContaining({ tags: ['dev'] })
				)
			);
		});

		it('should refuse a tag the CLI would read as a port', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(
				buildImportPreview({ candidates: [buildImportCandidate()] })
			);
			await analyzeScript(user);
			const field = screen.getByRole('combobox', { name: 'Tag to add' });

			await user.type(field, '3333');
			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));

			expect(field).toHaveAccessibleDescription(
				expect.stringContaining(i18n._(DIGITS_TAG_MESSAGE))
			);
			expect(field).toHaveFocus();
			expect(gateway.createTunnel).not.toHaveBeenCalled();
		});

		it('should create the selected tunnels in order with the tags', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(
				buildImportPreview({
					candidates: [
						buildImportCandidate({ listenPort: 3333, targetPort: 3333 }),
						buildImportCandidate({ listenPort: 4444, targetPort: 80 }),
					],
				})
			);
			await analyzeScript(user);

			await user.type(
				screen.getByRole('combobox', {
					name: 'Tag to add',
				}),
				'dev, imported'
			);
			await user.click(
				screen.getByRole('button', { name: 'Import 2 tunnels' })
			);

			await waitFor(() =>
				expect(gateway.createTunnel).toHaveBeenCalledTimes(2)
			);
			expect(gateway.createTunnel.mock.calls.map(([spec]) => spec)).toEqual([
				{
					host: 'nova',
					kind: 'local',
					listenPort: 3333,
					tags: ['dev', 'imported'],
				},
				{
					host: 'nova',
					kind: 'local',
					listenPort: 4444,
					targetPort: 80,
					tags: ['dev', 'imported'],
				},
			]);
		});

		it('should toast and close the drawer when everything is imported', async () => {
			const user = userEvent.setup();
			renderDrawer();
			await analyzeScript(user);

			await user.click(
				screen.getByRole('button', { name: 'Import 8 tunnels' })
			);

			await waitFor(() =>
				expect(useUiStore.getState().panel).toEqual({ kind: 'none' })
			);
			expect(getShownToasts().map((toast) => toast.message)).toEqual([
				'8 tunnels imported',
			]);
		});

		it('should keep the drawer, list the failures and unselect the created ones', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(
				buildImportPreview({
					candidates: [
						buildImportCandidate({ listenPort: 3333 }),
						buildImportCandidate({ listenPort: 4444 }),
					],
				})
			);
			gateway.createTunnel.mockResolvedValueOnce(buildTunnel());
			gateway.createTunnel.mockRejectedValueOnce(
				new DaemonCommandError('validation_failed', 'invalide', [
					'Port 4444 is already in use',
				])
			);
			await analyzeScript(user);

			await user.click(
				screen.getByRole('button', { name: 'Import 2 tunnels' })
			);

			const summary = await findFilledLiveRegion('alert');
			expect(summary).toHaveTextContent('1 tunnel imported, 1 failed');
			expect(summary).toHaveTextContent(
				'4444 → nova:4444: Port 4444 is already in use'
			);
			expect(summary).toHaveFocus();
			expect(
				screen.getByRole('checkbox', { name: '3333 to nova:3333' })
			).not.toBeChecked();
			expect(
				screen.getByRole('checkbox', { name: '4444 to nova:4444' })
			).toBeChecked();
			expect(useUiStore.getState().panel).toEqual({ kind: 'import' });
		});

		it('should show the progress while creating', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(
				buildImportPreview({ candidates: [buildImportCandidate()] })
			);
			let finishCreation: () => void = () => undefined;
			gateway.createTunnel.mockImplementation(
				() =>
					new Promise((resolve) => {
						finishCreation = () => resolve(buildTunnel());
					})
			);
			await analyzeScript(user);

			await user.click(screen.getByRole('button', { name: 'Import 1 tunnel' }));

			expect(await findFilledLiveRegion('status')).toHaveTextContent(
				'Import in progress: 0 of 1'
			);
			act(() => finishCreation());
			await waitFor(() =>
				expect(useUiStore.getState().panel).toEqual({ kind: 'none' })
			);
		});
	});

	it('should have no axe violations with the File tab selected', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await user.click(screen.getByRole('tab', { name: 'File' }));

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should have no axe violations in French with a file preview', async () => {
		await activateLocale('fr');
		const user = userEvent.setup();
		const { gateway } = renderDrawer();
		gateway.pickImportFile.mockResolvedValue({
			fileName: 'tunnels.toml',
			contents: 'CONTENTS',
		});
		gateway.previewConfigImport.mockResolvedValue(
			buildConfigImportPlan({
				unknownHosts: ['oldbox'],
				entries: [
					buildConfigImportEntry({
						fileHost: 'oldbox',
						warnings: [{ kind: 'unknownHost', host: 'oldbox' }],
					}),
					buildConfigImportEntry({
						index: 1,
						status: { kind: 'invalid', messages: ['port 0 is not valid'] },
						selectedByDefault: false,
					}),
				],
			})
		);

		await user.click(screen.getByRole('tab', { name: 'Fichier' }));
		await user.click(
			screen.getByRole('button', { name: 'Choisir un fichier…' })
		);
		await screen.findByRole('heading', { name: 'Aperçu de tunnels.toml' });

		expect(await axe(document.body)).toHaveNoViolations();
	});

	it('should have no axe violations', async () => {
		const user = userEvent.setup();
		renderDrawer(
			buildImportPreview({
				candidates: [
					buildImportCandidate(),
					buildImportCandidate({ host: undefined, listenPort: 4444 }),
				],
				warnings: [{ kind: 'unknownDestination', destination: 'x' }],
			})
		);
		await analyzeScript(user);

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
