import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { buildPaletteCommands } from '#/palette/build_palette_commands';
import { buildOpener, buildTunnel } from '#/testing/factories';

const labelsOf = (commands: ReturnType<typeof buildPaletteCommands>) =>
	commands.map((command) => command.label);

const build = (
	tunnels: Parameters<typeof buildPaletteCommands>[0]['tunnels'] = [],
	hosts: readonly string[] = [],
	openers: Parameters<typeof buildPaletteCommands>[0]['openers'] = []
) => buildPaletteCommands({ tunnels, hosts, openers }, i18n);

describe('buildPaletteCommands', () => {
	it('should always offer the general commands first', () => {
		expect(labelsOf(build())).toEqual([
			'New tunnel',
			'New temporary tunnel…',
			'Import tunnels',
			'Open settings',
			'Settings: General',
			'Settings: Connection',
			'Settings: Standby',
			'Settings: Opening',
			'Settings: Secrets',
			'Open the configuration folder',
			'Hide or show the sidebar',
			'Show keyboard shortcuts',
			'Open the documentation',
			'Check for updates',
		]);
	});

	it('should give the general commands the General category', () => {
		expect(new Set(build().map((command) => command.category))).toEqual(
			new Set(['General'])
		);
	});

	it('should offer one settings command per tab', () => {
		expect(build()).toContainEqual(
			expect.objectContaining({
				id: 'general:settings:secrets',
				category: 'General',
				action: { kind: 'openSettings', tab: 'secrets' },
			})
		);
	});

	it('should offer to start a stopped tunnel', () => {
		expect(build([buildTunnel()])).toContainEqual(
			expect.objectContaining({
				label: 'Start 3333 → nova:3333',
				category: 'Tunnel',
				action: { kind: 'startTunnel', tunnelId: 'tunnel-1' },
			})
		);
	});

	it('should offer to stop a running tunnel and not to start it', () => {
		const labels = labelsOf(
			build([buildTunnel({ state: { kind: 'active' } })])
		);

		expect(labels).toContain('Stop 3333 → nova:3333');
		expect(labels).not.toContain('Start 3333 → nova:3333');
	});

	it('should offer to put an awake tunnel back to standby', () => {
		expect(
			build([buildTunnel({ state: { kind: 'active' }, isAwake: true })])
		).toContainEqual(
			expect.objectContaining({
				label: 'Send 3333 → nova:3333 to standby',
				category: 'Tunnel',
				action: { kind: 'standbyTunnel', tunnelId: 'tunnel-1' },
			})
		);
	});

	it('should not offer standby for a tunnel that is not awake', () => {
		const labels = labelsOf(
			build([buildTunnel({ state: { kind: 'active' } })])
		);

		expect(labels).not.toContain('Send 3333 → nova:3333 to standby');
		expect(labels).not.toContain('Send awake tunnels to standby');
	});

	it('should offer to put every awake tunnel back to standby when one is awake', () => {
		expect(
			build([buildTunnel({ state: { kind: 'active' }, isAwake: true })])
		).toContainEqual(
			expect.objectContaining({
				label: 'Send awake tunnels to standby',
				category: 'General',
				action: { kind: 'standbyAll' },
			})
		);
	});

	it('should offer to put a tag back to standby only when one of its tunnels is awake', () => {
		const commands = build([
			buildTunnel({ id: 'a', tags: ['dev'], isAwake: true }),
			buildTunnel({ id: 'b', tags: ['db'] }),
		]);

		expect(commands).toContainEqual(
			expect.objectContaining({
				label: 'Send tag dev to standby',
				category: 'Tag',
				action: { kind: 'standbyTag', tag: 'dev' },
			})
		);
		expect(labelsOf(commands)).not.toContain('Send tag db to standby');
	});

	it('should offer details and edition for every tunnel', () => {
		const labels = labelsOf(build([buildTunnel()]));

		expect(labels).toContain('Details of 3333 → nova:3333');
		expect(labels).toContain('Edit 3333 → nova:3333');
	});

	it('should offer to open a local tunnel whether it runs or not', () => {
		expect(build([buildTunnel()])).toContainEqual(
			expect.objectContaining({
				label: 'Open 3333 → nova:3333 in the browser',
				action: { kind: 'openTunnel', id: 'tunnel-1' },
			})
		);
	});

	it('should offer the applications as other ways to open a local tunnel', () => {
		const commands = build([buildTunnel()], [], [buildOpener()]);

		expect(commands).toContainEqual(
			expect.objectContaining({
				label: 'Open 3333 → nova:3333 with Remmina (VNC)',
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
			'Open 3333 → nova:3333 with Remmina (VNC)',
			'Open 3333 → nova:3333 in the browser',
		]);
	});

	it('should not offer to open a remote tunnel', () => {
		expect(labelsOf(build([buildTunnel({ kind: 'remote' })]))).not.toContain(
			'Open 3333 → nova:3333 in the browser'
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
			expect.objectContaining({ label: 'Rename tag db…' }),
			expect.objectContaining({ label: 'Rename tag dev…' }),
		]);
		expect(
			commands.filter(({ action }) => action.kind === 'deleteTag')
		).toEqual([
			expect.objectContaining({ label: 'Delete tag db…' }),
			expect.objectContaining({ label: 'Delete tag dev…' }),
		]);
	});

	it('should offer to start a tag that is not fully active', () => {
		const labels = labelsOf(build([buildTunnel({ tags: ['dev'] })]));

		expect(labels).toContain('Start tag dev');
		expect(labels).not.toContain('Stop tag dev');
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

		expect(labels).toContain('Stop tag dev');
		expect(labels).not.toContain('Start tag dev');
	});

	it('should offer both for a tag mixing standby and stopped tunnels', () => {
		const labels = labelsOf(
			build([
				buildTunnel({ tags: ['dev'], state: { kind: 'standby' } }),
				buildTunnel({ id: 'tunnel-2', tags: ['dev'] }),
			])
		);

		expect(labels).toContain('Stop tag dev');
		expect(labels).toContain('Start tag dev');
	});

	it('should offer to stop a tag with an active tunnel', () => {
		const labels = labelsOf(
			build([
				buildTunnel({ tags: ['dev'], state: { kind: 'active' } }),
				buildTunnel({ id: 'tunnel-2', tags: ['dev'] }),
			])
		);

		expect(labels).toContain('Stop tag dev');
		expect(labels).toContain('Start tag dev');
	});

	it('should only offer to stop a fully active tag', () => {
		const labels = labelsOf(
			build([buildTunnel({ tags: ['dev'], state: { kind: 'active' } })])
		);

		expect(labels).toContain('Stop tag dev');
		expect(labels).not.toContain('Start tag dev');
	});

	it('should offer the host commands for the known hosts and the tunnel hosts, sorted', () => {
		const labels = labelsOf(
			build([buildTunnel({ host: 'zeta' })], ['nova', 'Alpha'])
		);

		expect(
			labels.filter((label) => label.includes('Configuration of'))
		).toEqual([
			'Configuration of Alpha',
			'Configuration of nova',
			'Configuration of zeta',
		]);
		expect(labels).toContain('X11 for zeta');
	});

	it('should not repeat a host known twice', () => {
		const labels = labelsOf(build([buildTunnel({ host: 'nova' })], ['nova']));

		expect(labels.filter((label) => label === 'X11 for nova')).toHaveLength(1);
	});

	it('should give every command a distinct id', () => {
		const ids = build(
			[buildTunnel({ tags: ['dev'] }), buildTunnel({ id: 'tunnel-2' })],
			['nova']
		).map((command) => command.id);

		expect(new Set(ids).size).toBe(ids.length);
	});
});
