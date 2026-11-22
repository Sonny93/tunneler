import { describe, expect, it } from 'vitest';

import type { ImportConflictDto } from '#/daemon/protocol';
import { flattenDevslotPorts } from '#/import/devslot/devslot_ports';
import {
	buildDevslotClassifyRequest,
	findDevslotProblems,
	type DevslotLocalPorts,
} from '#/import/devslot/find_devslot_problems';
import { getInitialDevslotSelection } from '#/import/devslot/get_initial_devslot_selection';
import { buildDevslotInstance, buildDevslotPort } from '#/testing/factories';

function buildEntries(ports = [buildDevslotPort()]) {
	return flattenDevslotPorts([buildDevslotInstance({ ports })]);
}

type ProblemsCase = {
	ports?: ReturnType<typeof buildDevslotPort>[];
	localPorts?: DevslotLocalPorts;
	selected?: string[];
	conflicts?: Record<string, ImportConflictDto | null>;
};

function findProblems({
	ports,
	localPorts = {},
	selected = ['0:0'],
	conflicts = {},
}: ProblemsCase = {}) {
	const entries = buildEntries(ports);
	const request = buildDevslotClassifyRequest({
		entries,
		localPorts,
		selectedKeys: new Set(selected),
		host: 'vega',
	});
	return findDevslotProblems(
		{ entries, localPorts },
		request,
		new Map(Object.entries(conflicts))
	);
}

describe('buildDevslotClassifyRequest', () => {
	it('should put the chosen ports first, then the others, each with its local route', () => {
		const entries = buildEntries([
			buildDevslotPort({ var: 'A', port: 20_050 }),
			buildDevslotPort({ var: 'B', port: 20_051 }),
			buildDevslotPort({ var: 'C', port: 20_052 }),
		]);

		const request = buildDevslotClassifyRequest({
			entries,
			localPorts: { '0:2': '30052' },
			selectedKeys: new Set(['0:2', '0:1']),
			host: 'vega',
		});

		expect(request.keys).toEqual(['0:1', '0:2', '0:0']);
		expect(request.chosenKeys).toEqual(['0:1', '0:2']);
		expect(request.routes[1]).toEqual({
			host: 'vega',
			kind: 'local',
			listenPort: 30_052,
			listenAddress: '127.0.0.1',
			targetPort: 20_052,
		});
	});

	it('should leave out a port whose local port cannot be read and one already tunneled', () => {
		const entries = buildEntries([
			buildDevslotPort({ var: 'A' }),
			buildDevslotPort({ var: 'B', status: 'alreadyTunneled' }),
		]);

		const request = buildDevslotClassifyRequest({
			entries,
			localPorts: { '0:0': 'abc' },
			selectedKeys: new Set(),
			host: 'vega',
		});

		expect(request.keys).toEqual([]);
	});
});

describe('findDevslotProblems', () => {
	it('should report the conflict the daemon found for a port', () => {
		const problems = findProblems({
			conflicts: { '0:0': { kind: 'listenConflict', holderId: 'saved' } },
		});

		expect(problems.get('0:0')).toEqual({
			kind: 'listenConflict',
			holderId: 'saved',
		});
	});

	it('should report no problem before the daemon answered', () => {
		expect(findProblems().get('0:0')).toBeNull();
	});

	it('should report an invalid local port', () => {
		const problems = findProblems({ localPorts: { '0:0': 'abc' } });

		expect(problems.get('0:0')).toEqual({ kind: 'invalidLocalPort' });
	});

	it('should report a busy port without suggestion while its local port is the remote one', () => {
		const ports = [buildDevslotPort({ localPortBusy: true })];

		const unchanged = findProblems({ ports });
		const changed = findProblems({ ports, localPorts: { '0:0': '30050' } });

		expect(unchanged.get('0:0')).toEqual({
			kind: 'localPortBusy',
			port: 20_050,
		});
		expect(changed.get('0:0')).toBeNull();
	});

	it('should keep a local problem over a conflict', () => {
		const problems = findProblems({
			ports: [buildDevslotPort({ localPortBusy: true })],
			conflicts: { '0:0': { kind: 'duplicateInImport' } },
		});

		expect(problems.get('0:0')).toEqual({
			kind: 'localPortBusy',
			port: 20_050,
		});
	});

	it('should accept a busy port that has a suggestion', () => {
		const problems = findProblems({
			ports: [
				buildDevslotPort({ localPortBusy: true, suggestedLocalPort: 30_050 }),
			],
		});

		expect(problems.get('0:0')).toBeNull();
	});

	it('should ignore a port that is already tunneled', () => {
		const problems = findProblems({
			ports: [buildDevslotPort({ status: 'alreadyTunneled' })],
			selected: [],
		});

		expect(problems.has('0:0')).toBe(false);
	});
});

describe('getInitialDevslotSelection', () => {
	it('should check the preselected ports and leave out a busy one without alternative', () => {
		const entries = buildEntries([
			buildDevslotPort({ var: 'A', port: 20_050 }),
			buildDevslotPort({ var: 'B', port: 20_051, localPortBusy: true }),
			buildDevslotPort({ var: 'C', port: 20_052, kind: 'service' }),
		]);

		expect([...getInitialDevslotSelection(entries)]).toEqual(['0:0']);
	});
});
