import { describe, expect, it } from 'vitest';

import { buildSettings, buildTunnel } from '#/testing/factories';
import { buildPaletteCommands } from '#/palette/build_palette_commands';

const SETTINGS = buildSettings();

const labelsOf = (commands: ReturnType<typeof buildPaletteCommands>) =>
	commands.map((command) => command.label);

const build = (
	tunnels: Parameters<typeof buildPaletteCommands>[0]['tunnels'] = [],
	hosts: readonly string[] = []
) => buildPaletteCommands({ tunnels, hosts, settings: SETTINGS });

describe('buildPaletteCommands', () => {
	it('should always offer the general commands first', () => {
		expect(labelsOf(build())).toEqual([
			'Nouveau tunnel',
			'Importer des tunnels',
			'Ouvrir les paramètres',
			'Ouvrir le dossier de configuration',
			'Masquer ou afficher le panneau',
			'Afficher les raccourcis clavier',
		]);
	});

	it('should give the general commands the Général category', () => {
		expect(new Set(build().map((command) => command.category))).toEqual(
			new Set(['Général'])
		);
	});

	it('should offer to start a stopped tunnel', () => {
		expect(build([buildTunnel()])).toContainEqual(
			expect.objectContaining({
				label: 'Démarrer Adonis',
				category: 'Tunnel',
				action: { kind: 'startTunnel', tunnelId: 'tunnel-1' },
			})
		);
	});

	it('should offer to stop a running tunnel and not to start it', () => {
		const labels = labelsOf(
			build([buildTunnel({ state: { kind: 'active' } })])
		);

		expect(labels).toContain('Arrêter Adonis');
		expect(labels).not.toContain('Démarrer Adonis');
	});

	it('should offer details and edition for every tunnel', () => {
		const labels = labelsOf(build([buildTunnel()]));

		expect(labels).toContain('Détails de Adonis');
		expect(labels).toContain('Modifier Adonis');
	});

	it('should offer the browser for a running local tunnel only', () => {
		const running = { state: { kind: 'active' } } as const;

		expect(build([buildTunnel(running)])).toContainEqual(
			expect.objectContaining({
				label: 'Ouvrir Adonis dans le navigateur',
				action: { kind: 'openInBrowser', host: '127.0.0.1', port: 3333 },
			})
		);
		expect(labelsOf(build([buildTunnel()]))).not.toContain(
			'Ouvrir Adonis dans le navigateur'
		);
		expect(
			labelsOf(build([buildTunnel({ ...running, kind: 'remote' })]))
		).not.toContain('Ouvrir Adonis dans le navigateur');
	});

	it('should offer to rename every tag', () => {
		const commands = build([
			buildTunnel({ tags: ['dev'] }),
			buildTunnel({ id: 'tunnel-2', tags: ['dev', 'db'] }),
		]);

		expect(
			commands.filter(({ action }) => action.kind === 'renameTag')
		).toEqual([
			expect.objectContaining({ label: 'Renommer le tag db…' }),
			expect.objectContaining({ label: 'Renommer le tag dev…' }),
		]);
	});

	it('should offer to delete every tag', () => {
		const commands = build([
			buildTunnel({ tags: ['dev'] }),
			buildTunnel({ id: 'tunnel-2', tags: ['dev', 'db'] }),
		]);

		expect(
			commands.filter(({ action }) => action.kind === 'deleteTag')
		).toEqual([
			expect.objectContaining({ label: 'Supprimer le tag db…' }),
			expect.objectContaining({ label: 'Supprimer le tag dev…' }),
		]);
	});

	it('should offer to start a tag that is not fully active', () => {
		const labels = labelsOf(build([buildTunnel({ tags: ['dev'] })]));

		expect(labels).toContain('Démarrer le tag dev');
		expect(labels).not.toContain('Arrêter le tag dev');
	});

	it('should offer to stop a tag with an active tunnel', () => {
		const labels = labelsOf(
			build([
				buildTunnel({ tags: ['dev'], state: { kind: 'active' } }),
				buildTunnel({ id: 'tunnel-2', tags: ['dev'] }),
			])
		);

		expect(labels).toContain('Arrêter le tag dev');
		expect(labels).toContain('Démarrer le tag dev');
	});

	it('should only offer to stop a fully active tag', () => {
		const labels = labelsOf(
			build([buildTunnel({ tags: ['dev'], state: { kind: 'active' } })])
		);

		expect(labels).toContain('Arrêter le tag dev');
		expect(labels).not.toContain('Démarrer le tag dev');
	});

	it('should offer the host commands for the known hosts and the tunnel hosts, sorted', () => {
		const labels = labelsOf(
			build([buildTunnel({ host: 'zeta' })], ['nova', 'Alpha'])
		);

		expect(
			labels.filter((label) => label.includes('Configuration de'))
		).toEqual([
			'Configuration de Alpha',
			'Configuration de nova',
			'Configuration de zeta',
		]);
		expect(labels).toContain('X11 pour zeta');
	});

	it('should not repeat a host known twice', () => {
		const labels = labelsOf(build([buildTunnel({ host: 'nova' })], ['nova']));

		expect(labels.filter((label) => label === 'X11 pour nova')).toHaveLength(1);
	});

	it('should give every command a distinct id', () => {
		const ids = build(
			[buildTunnel({ tags: ['dev'] }), buildTunnel({ id: 'tunnel-2' })],
			['nova']
		).map((command) => command.id);

		expect(new Set(ids).size).toBe(ids.length);
	});
});
