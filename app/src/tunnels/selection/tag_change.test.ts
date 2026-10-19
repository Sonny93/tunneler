import { describe, expect, it } from 'vitest';

import { buildTunnel } from '#/testing/factories';
import {
	applyTagChange,
	findTunnelsToRetag,
	toRetaggedDefinition,
} from '#/tunnels/selection/tag_change';

describe('applyTagChange', () => {
	it('should append a new tag', () => {
		expect(applyTagChange(['dev'], { kind: 'add', tags: ['db'] })).toEqual([
			'dev',
			'db',
		]);
	});

	it('should not duplicate a tag that is already there', () => {
		expect(applyTagChange(['dev'], { kind: 'add', tags: ['dev'] })).toEqual([
			'dev',
		]);
	});

	it('should remove a tag', () => {
		expect(
			applyTagChange(['dev', 'db'], { kind: 'remove', tag: 'dev' })
		).toEqual(['db']);
	});
});

describe('findTunnelsToRetag', () => {
	it('should skip the tunnels the change leaves unchanged', () => {
		const tunnels = [
			buildTunnel({ id: 'a', tags: ['dev'] }),
			buildTunnel({ id: 'b', tags: [] }),
		];

		expect(
			findTunnelsToRetag(tunnels, { kind: 'add', tags: ['dev'] }).map(
				({ id }) => id
			)
		).toEqual(['b']);
	});
});

describe('toRetaggedDefinition', () => {
	it('should keep the definition of the tunnel and replace its tags', () => {
		const tunnel = buildTunnel({
			id: 'a',
			kind: 'local',
			targetHost: 'db',
			targetPort: 5432,
			listenAddress: '0.0.0.0',
			tags: ['dev'],
		});

		expect(toRetaggedDefinition(tunnel, { kind: 'add', tags: ['db'] })).toEqual(
			{
				id: 'a',
				host: 'nova',
				kind: 'local',
				listenPort: 3333,
				listenAddress: '0.0.0.0',
				targetHost: 'db',
				targetPort: 5432,
				tags: ['dev', 'db'],
			}
		);
	});

	it('should not send the runtime state of the tunnel', () => {
		const definition = toRetaggedDefinition(buildTunnel(), {
			kind: 'add',
			tags: ['dev'],
		});

		expect(definition).not.toHaveProperty('state');
	});
});
