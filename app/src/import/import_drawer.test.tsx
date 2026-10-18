import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { act, screen, waitFor, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { ImportDrawer } from '#/import/import_drawer';
import type { ImportPreviewDto } from '#/daemon/protocol';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import {
	buildImportCandidate,
	buildImportPreview,
	buildSettings,
	buildTunnel,
} from '#/testing/factories';

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
			name: /Script, liste de ports ou options -L\/-R\/-D/,
		}),
		SCRIPT
	);
	await user.click(screen.getByRole('button', { name: 'Analyser' }));
	await screen.findByRole('heading', { name: "Résultat de l'analyse" });
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
			screen.getByRole('dialog', { name: 'Importer des tunnels' })
		).toBeInTheDocument();
		expect(
			screen.getByRole('tab', { name: 'Coller un script' })
		).toBeInTheDocument();
		expect(
			screen.getByRole('tab', { name: 'Depuis ~/.ssh/config' })
		).toBeInTheDocument();
	});

	it('should describe the accepted input', () => {
		renderDrawer();

		expect(
			screen.getByRole('textbox', {
				name: /Script, liste de ports ou options -L\/-R\/-D/,
			})
		).toHaveAccessibleDescription(/liste de ports \(3333, 5432\)/);
	});

	it('should link an error to the empty script field', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('button', { name: 'Analyser' }));

		const field = screen.getByRole('textbox', {
			name: /Script, liste de ports ou options -L\/-R\/-D/,
		});
		expect(field).toBeInvalid();
		expect(field).toHaveAccessibleDescription(/Collez un script/);
		expect(gateway.importScript).not.toHaveBeenCalled();
	});

	it('should analyze the script and announce the detected tunnels', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await analyzeScript(user);

		expect(gateway.importScript).toHaveBeenCalledWith(SCRIPT);
		expect(screen.getByText('8 tunnels détectés')).toBeInTheDocument();
		expect(useAnnouncerStore.getState().polite.message).toBe(
			'8 tunnels détectés'
		);
	});

	it('should move the focus to the preview heading', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await analyzeScript(user);

		expect(
			screen.getByRole('heading', { name: "Résultat de l'analyse" })
		).toHaveFocus();
	});

	it('should read ~/.ssh/config from its tab', async () => {
		const user = userEvent.setup();
		const { gateway } = renderDrawer();

		await user.click(screen.getByRole('tab', { name: 'Depuis ~/.ssh/config' }));
		await user.click(
			screen.getByRole('button', { name: 'Lire ~/.ssh/config' })
		);

		expect(
			await screen.findByRole('group', { name: 'Tunnels détectés (8)' })
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
		await user.click(screen.getByRole('button', { name: 'Analyser' }));

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent('Analyse impossible :');
		expect(alert).toHaveTextContent('script trop long');
		expect(alert).toHaveFocus();
	});

	it('should check every candidate and name them after host and port', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await analyzeScript(user);

		expect(screen.getAllByRole('checkbox')).toHaveLength(8);
		expect(screen.getByRole('checkbox', { name: /nova 3333/ })).toBeChecked();
		expect(
			screen.getByRole('button', { name: 'Importer 8 tunnels' })
		).toBeEnabled();
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

		const section = screen.getByRole('region', { name: 'Avertissements' });
		expect(section).toHaveTextContent(
			'Ligne 3 : option ignorée, illisible : -L nope'
		);
		expect(section).toHaveTextContent(
			'Ligne 5 : nova : forward non pris en charge (socket Unix ou SOCKS distant) : -L /tmp/s:x:1'
		);
		expect(section).toHaveTextContent(
			"Aucun alias ssh_config ne correspond à user@198.51.100.10 : choisissez l'hôte ci-dessous."
		);
		expect(section).toHaveTextContent('Lecture de old impossible : bad config');
		expect(within(section).getByText('bad config')).toHaveAttribute(
			'lang',
			'en'
		);
	});

	it('should uncheck a candidate in conflict and say why', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({ tunnels: [buildTunnel({ name: 'Adonis' })] });
		renderDrawer();

		await analyzeScript(user);

		const conflicting = screen.getByRole('checkbox', { name: /nova 3333/ });
		expect(conflicting).not.toBeChecked();
		expect(conflicting).toHaveAccessibleDescription(
			/Déjà configuré \(Adonis\)/
		);
		expect(
			screen.getByRole('button', { name: 'Importer 7 tunnels' })
		).toBeInTheDocument();
	});

	it('should let the user check a candidate in conflict', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({ tunnels: [buildTunnel({ name: 'Adonis' })] });
		renderDrawer();
		await analyzeScript(user);

		await user.click(screen.getByRole('checkbox', { name: /nova 3333/ }));

		expect(
			screen.getByRole('button', { name: 'Importer 8 tunnels' })
		).toBeInTheDocument();
	});

	it('should select and unselect everything', async () => {
		const user = userEvent.setup();
		renderDrawer();
		await analyzeScript(user);

		await user.click(
			screen.getByRole('button', { name: 'Tout désélectionner' })
		);
		expect(
			screen.getByRole('button', { name: 'Importer 0 tunnels' })
		).toBeDisabled();

		await user.click(screen.getByRole('button', { name: 'Tout sélectionner' }));
		expect(
			screen.getByRole('button', { name: 'Importer 8 tunnels' })
		).toBeEnabled();
	});

	it('should suffix a proposed name already taken', async () => {
		const user = userEvent.setup();
		useDaemonStore.setState({
			tunnels: [buildTunnel({ name: 'NOVA 3334', listenPort: 9999 })],
		});
		renderDrawer();

		await analyzeScript(user);

		expect(
			screen.getByRole('checkbox', { name: 'nova 3334 2' })
		).toBeInTheDocument();
	});

	describe('candidates without a host', () => {
		const hostlessPreview = () =>
			buildImportPreview({
				candidates: [
					buildImportCandidate({ host: undefined, listenPort: 5000 }),
				],
			});

		it('should require a host choice before importing', async () => {
			const user = userEvent.setup();
			const { gateway } = renderDrawer(hostlessPreview());
			await analyzeScript(user);

			await user.click(
				screen.getByRole('button', { name: 'Importer 1 tunnel' })
			);

			const select = screen.getByRole('combobox', {
				name: /Hôte des tunnels sans destination/,
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
					name: /Hôte des tunnels sans destination/,
				}),
				'other'
			);
			await user.click(
				screen.getByRole('button', { name: 'Importer 1 tunnel' })
			);

			await waitFor(() =>
				expect(gateway.createTunnel).toHaveBeenCalledWith({
					name: 'ssh 5000',
					host: 'other',
					kind: 'local',
					listenPort: 5000,
					tags: [],
				})
			);
		});
	});

	it('should not ask for a host when every candidate has one', async () => {
		const user = userEvent.setup();
		renderDrawer();

		await analyzeScript(user);

		expect(
			screen.queryByRole('combobox', { name: /Hôte des tunnels/ })
		).not.toBeInTheDocument();
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

			const field = screen.getByRole('combobox', { name: 'Tag à ajouter' });

			expect(field).toHaveAccessibleDescription(/Optionnel/);
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

			await user.click(
				screen.getByRole('button', { name: 'Importer 1 tunnel' })
			);

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
				screen.getByRole('combobox', { name: 'Tag à ajouter' }),
				' dev , dev'
			);
			await user.click(
				screen.getByRole('button', { name: 'Importer 1 tunnel' })
			);

			await waitFor(() =>
				expect(gateway.createTunnel).toHaveBeenCalledWith(
					expect.objectContaining({ tags: ['dev'] })
				)
			);
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
					name: 'Tag à ajouter',
				}),
				'dev, imported'
			);
			await user.click(
				screen.getByRole('button', { name: 'Importer 2 tunnels' })
			);

			await waitFor(() =>
				expect(gateway.createTunnel).toHaveBeenCalledTimes(2)
			);
			expect(gateway.createTunnel.mock.calls.map(([spec]) => spec)).toEqual([
				{
					name: 'nova 3333',
					host: 'nova',
					kind: 'local',
					listenPort: 3333,
					tags: ['dev', 'imported'],
				},
				{
					name: 'nova 4444',
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
				screen.getByRole('button', { name: 'Importer 8 tunnels' })
			);

			await waitFor(() =>
				expect(useUiStore.getState().panel).toEqual({ kind: 'none' })
			);
			expect(
				useToastStore.getState().toasts.map((toast) => toast.message)
			).toEqual(['8 tunnels importés']);
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
					'Le port 4444 est déjà utilisé',
				])
			);
			await analyzeScript(user);

			await user.click(
				screen.getByRole('button', { name: 'Importer 2 tunnels' })
			);

			const summary = await screen.findByRole('alert');
			expect(summary).toHaveTextContent('1 tunnel importé, 1 en échec');
			expect(summary).toHaveTextContent(
				'nova 4444 : Le port 4444 est déjà utilisé'
			);
			expect(summary).toHaveFocus();
			expect(
				screen.getByRole('checkbox', { name: 'nova 3333' })
			).not.toBeChecked();
			expect(screen.getByRole('checkbox', { name: 'nova 4444' })).toBeChecked();
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

			await user.click(
				screen.getByRole('button', { name: 'Importer 1 tunnel' })
			);

			expect(await screen.findByRole('status')).toHaveTextContent(
				'Import en cours : 0 sur 1'
			);
			act(() => finishCreation());
			await waitFor(() =>
				expect(useUiStore.getState().panel).toEqual({ kind: 'none' })
			);
		});
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
