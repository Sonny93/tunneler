import { describe, expect, it } from 'vitest';

import {
	buildSavedDevslotSpecs,
	buildTemporaryDevslotSpecs,
} from '#/import/devslot/build_devslot_specs';
import { flattenDevslotPorts } from '#/import/devslot/devslot_ports';
import { buildDevslotInstance, buildDevslotPort } from '#/testing/factories';

function buildInput(
	ports = [buildDevslotPort()],
	overrides: {
		localPorts?: Record<string, string>;
		projectTag?: string | null;
	} = {}
) {
	const projectTag =
		overrides.projectTag === null
			? {}
			: { projectTag: overrides.projectTag ?? 'ds-web-shop' };
	const entries = flattenDevslotPorts([
		{
			...buildDevslotInstance({ ports }),
			projectTag: undefined,
			...projectTag,
		},
	]);
	return {
		entries,
		selectedKeys: new Set(entries.map(({ key }) => key)),
		localPorts: overrides.localPorts ?? {},
		host: 'vega',
	};
}

describe('buildTemporaryDevslotSpecs', () => {
	it('should listen on 127.0.0.1 and omit the target port when equal to the local one', () => {
		const [item] = buildTemporaryDevslotSpecs(buildInput());

		expect(item?.spec).toEqual({
			host: 'vega',
			kind: 'local',
			listenPort: 20_050,
			listenAddress: '127.0.0.1',
			devslot: { projectTag: 'ds-web-shop' },
		});
	});

	it('should carry no tags', () => {
		const [item] = buildTemporaryDevslotSpecs(buildInput());

		expect(item?.spec).not.toHaveProperty('tags');
	});

	it('should send an empty origin when the instance has no project tag', () => {
		const [item] = buildTemporaryDevslotSpecs(
			buildInput(undefined, { projectTag: null })
		);

		expect(item?.spec.devslot).toEqual({});
	});

	it('should target the remote port when the local port was changed', () => {
		const [item] = buildTemporaryDevslotSpecs(
			buildInput(undefined, { localPorts: { '0:0': '30050' } })
		);

		expect(item?.spec).toMatchObject({
			listenPort: 30_050,
			targetPort: 20_050,
		});
	});

	it('should set the open host name of a web port that has one', () => {
		const [item] = buildTemporaryDevslotSpecs(
			buildInput([buildDevslotPort({ hostName: 'app.localhost' })])
		);

		expect(item?.spec.open).toEqual({ hostName: 'app.localhost' });
	});

	it('should send no open options without a host name or for a service', () => {
		const items = buildTemporaryDevslotSpecs(
			buildInput([
				buildDevslotPort(),
				buildDevslotPort({ kind: 'service', hostName: 'db.localhost' }),
			])
		);

		expect(items.map(({ spec }) => spec.open)).toEqual([undefined, undefined]);
	});

	it('should skip the ports that are not selected', () => {
		const input = { ...buildInput(), selectedKeys: new Set<string>() };

		expect(buildTemporaryDevslotSpecs(input)).toEqual([]);
	});
});

describe('buildSavedDevslotSpecs', () => {
	it('should tag the tunnel with devslot and the project tag', () => {
		const [item] = buildSavedDevslotSpecs(buildInput());

		expect(item?.spec).toMatchObject({
			listenAddress: '127.0.0.1',
			tags: ['devslot', 'ds-web-shop'],
		});
	});

	it('should only carry the devslot tag without a project tag', () => {
		const [item] = buildSavedDevslotSpecs(
			buildInput(undefined, { projectTag: null })
		);

		expect(item?.spec.tags).toEqual(['devslot']);
	});

	it('should keep the open host name', () => {
		const [item] = buildSavedDevslotSpecs(
			buildInput([buildDevslotPort({ hostName: 'app.localhost' })])
		);

		expect(item?.spec.open).toEqual({ hostName: 'app.localhost' });
	});
});
