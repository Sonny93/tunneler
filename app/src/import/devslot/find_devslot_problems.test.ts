import { describe, expect, it } from 'vitest';

import { flattenDevslotPorts } from '#/import/devslot/devslot_ports';
import { findDevslotProblems } from '#/import/devslot/find_devslot_problems';
import { getInitialDevslotSelection } from '#/import/devslot/get_initial_devslot_selection';
import {
	buildDevslotInstance,
	buildDevslotPort,
	buildSettings,
	buildTunnel,
} from '#/testing/factories';

const settings = buildSettings();

function buildEntries(ports = [buildDevslotPort()]) {
	return flattenDevslotPorts([buildDevslotInstance({ ports })]);
}

describe('findDevslotProblems', () => {
	it('should report a chosen port whose local port is used by another tunnel', () => {
		const problems = findDevslotProblems({
			entries: buildEntries(),
			localPorts: {},
			selectedKeys: new Set(['0:0']),
			tunnels: [
				buildTunnel({ host: 'vega', listenPort: 20_050, targetPort: 1 }),
			],
			settings,
			host: 'vega',
		});

		expect(problems.get('0:0')).toMatchObject({ kind: 'listenConflict' });
	});

	it('should report an invalid local port', () => {
		const problems = findDevslotProblems({
			entries: buildEntries(),
			localPorts: { '0:0': 'abc' },
			selectedKeys: new Set(['0:0']),
			tunnels: [],
			settings,
			host: 'vega',
		});

		expect(problems.get('0:0')).toEqual({ kind: 'invalidLocalPort' });
	});

	it('should accept an edited local port that is free', () => {
		const problems = findDevslotProblems({
			entries: buildEntries(),
			localPorts: { '0:0': '30050' },
			selectedKeys: new Set(['0:0']),
			tunnels: [
				buildTunnel({ host: 'vega', listenPort: 20_050, targetPort: 1 }),
			],
			settings,
			host: 'vega',
		});

		expect(problems.get('0:0')).toBeNull();
	});

	it('should only make the chosen ports clash with each other', () => {
		const entries = buildEntries([
			buildDevslotPort({ var: 'A', port: 20_050 }),
			buildDevslotPort({ var: 'B', port: 20_050 }),
		]);
		const common = {
			entries,
			localPorts: {},
			tunnels: [],
			settings,
			host: 'vega',
		};

		const both = findDevslotProblems({
			...common,
			selectedKeys: new Set(['0:0', '0:1']),
		});
		const one = findDevslotProblems({
			...common,
			selectedKeys: new Set(['0:1']),
		});

		expect(both.get('0:1')).toEqual({ kind: 'duplicateInImport' });
		expect(one.get('0:1')).toBeNull();
	});

	it('should ignore a port that is already tunneled', () => {
		const problems = findDevslotProblems({
			entries: buildEntries([buildDevslotPort({ status: 'alreadyTunneled' })]),
			localPorts: {},
			selectedKeys: new Set(),
			tunnels: [],
			settings,
			host: 'vega',
		});

		expect(problems.has('0:0')).toBe(false);
	});
});

describe('getInitialDevslotSelection', () => {
	it('should leave out a preselected port that clashes with an existing tunnel', () => {
		const entries = buildEntries([
			buildDevslotPort({ var: 'A', port: 20_050 }),
			buildDevslotPort({ var: 'B', port: 20_051 }),
		]);
		const tunnels = [
			buildTunnel({ host: 'vega', listenPort: 20_050, targetPort: 1 }),
		];

		const selection = getInitialDevslotSelection(
			entries,
			tunnels,
			settings,
			'vega'
		);

		expect([...selection]).toEqual(['0:1']);
	});
});
