import { describe, expect, it } from 'vitest';

import { buildOpener, buildTunnel } from '#/testing/factories';
import { buildPaletteCommands } from '#/palette/build_palette_commands';

const labelsOf = (commands: ReturnType<typeof buildPaletteCommands>) =>
	commands.map((command) => command.label);

const build = (
	tunnels: Parameters<typeof buildPaletteCommands>[0]['tunnels'] = [],
	hosts: readonly string[] = [],
	openers: Parameters<typeof buildPaletteCommands>[0]['openers'] = []
) => buildPaletteCommands({ tunnels, hosts, openers });

describe('buildPaletteCommands', () => {
	it('should always offer the general commands first', () => {
		expect(labelsOf(build())).toEqual([
			'Nouveau tunnel',
			'Nouveau tunnel temporaire…',
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
				label: 'Démarrer 3333 → nova:3333',
				category: 'Tunnel',
				action: { kind: 'startTunnel', tunnelId: 'tunnel-1' },
			})
		);
	});

	it('should offer to stop a running tunnel and not to start it', () => {
		const labels = labelsOf(
			build([buildTunnel({ state: { kind: 'active' } })])
		);

		expect(labels).toContain('Arrêter 3333 → nova:3333');
		expect(labels).not.toContain('Démarrer 3333 → nova:3333');
	});

	it('should offer details and edition for every tunnel', () => {
		const labels = labelsOf(build([buildTunnel()]));

		expect(labels).toContain('Détails de 3333 → nova:3333');
		expect(labels).toContain('Modifier 3333 → nova:3333');
	});

	it('should offer to open a local tunnel whether it runs or not', () => {
		expect(build([buildTunnel()])).toContainEqual(
			expect.objectContaining({
				label: 'Ouvrir 3333 → nova:3333 dans le navigateur',
				action: { kind: 'openTunnel', id: 'tunnel-1' },
			})
		);
	});

	it('should offer the applications as other ways to open a local tunnel', () => {
		const commands = build([buildTunnel()], [], [buildOpener()]);

		expect(commands).toContainEqual(
			expect.objectContaining({
				label: 'Ouvrir 3333 → nova:3333 avec Remmina (VNC)',
				action: { kind: 'openTunnel', id: 'tunnel-1', opener: 'remmina-vnc' },
			})
		);
	});

	it('should open with the default application and offer the browser as an alternative', () => {
		const commands = build(
			[
				buildTunnel({
					defaultOpener: { id: 'remmina-vnc', name: 'Remmina (VNC)' },
				}),
			],
			[],
			[buildOpener()]
		);

		expect(
			commands
				.filter(({ action }) => action.kind === 'openTunnel')
				.map(({ label }) => label)
		).toEqual([
			'Ouvrir 3333 → nova:3333 avec Remmina (VNC)',
			'Ouvrir 3333 → nova:3333 dans le navigateur',
		]);
	});

	it('should not offer to open a remote tunnel', () => {
		expect(labelsOf(build([buildTunnel({ kind: 'remote' })]))).not.toContain(
			'Ouvrir 3333 → nova:3333 dans le navigateur'
		);
	});

	it('should offer to rename and delete every tag', () => {
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

	it('should only offer to stop a tag whose tunnels are all in standby', () => {
		const labels = labelsOf(
			build([
				buildTunnel({ tags: ['dev'], state: { kind: 'standby' } }),
				buildTunnel({
					id: 'tunnel-2',
					tags: ['dev'],
					state: { kind: 'standby' },
				}),
			])
		);

		expect(labels).toContain('Arrêter le tag dev');
		expect(labels).not.toContain('Démarrer le tag dev');
	});

	it('should offer both for a tag mixing standby and stopped tunnels', () => {
		const labels = labelsOf(
			build([
				buildTunnel({ tags: ['dev'], state: { kind: 'standby' } }),
				buildTunnel({ id: 'tunnel-2', tags: ['dev'] }),
			])
		);

		expect(labels).toContain('Arrêter le tag dev');
		expect(labels).toContain('Démarrer le tag dev');
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
