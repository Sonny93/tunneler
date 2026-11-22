import { describe, expect, it } from 'vitest';

import {
	flattenDevslotPorts,
	isPortPreselected,
	isPortTunnelable,
} from '#/import/devslot/devslot_ports';
import { buildDevslotInstance, buildDevslotPort } from '#/testing/factories';

function buildEntry(overrides: Parameters<typeof buildDevslotPort>[0]) {
	const [entry] = flattenDevslotPorts([
		buildDevslotInstance({ ports: [buildDevslotPort(overrides)] }),
	]);
	if (entry === undefined) throw new Error('no entry');
	return entry;
}

describe('flattenDevslotPorts', () => {
	it('should key each port by its instance and port positions', () => {
		const entries = flattenDevslotPorts([
			buildDevslotInstance({ ports: [buildDevslotPort(), buildDevslotPort()] }),
			buildDevslotInstance({ ports: [buildDevslotPort()] }),
		]);

		expect(entries.map(({ key }) => key)).toEqual(['0:0', '0:1', '1:0']);
	});
});

describe('isPortPreselected', () => {
	it('should preselect a web port', () => {
		expect(isPortPreselected(buildEntry({ kind: 'web' }))).toBe(true);
	});

	it('should not preselect a service port', () => {
		expect(isPortPreselected(buildEntry({ kind: 'service' }))).toBe(false);
	});

	it('should treat an unknown kind like a service', () => {
		expect(isPortPreselected(buildEntry({ kind: 'queue' }))).toBe(false);
	});

	it('should not preselect a web port below 1024', () => {
		expect(isPortPreselected(buildEntry({ port: 80 }))).toBe(false);
	});

	it('should not preselect a port that is already tunneled', () => {
		expect(isPortPreselected(buildEntry({ status: 'alreadyTunneled' }))).toBe(
			false
		);
	});
});

describe('isPortTunnelable', () => {
	it('should refuse an already tunneled port only', () => {
		expect(isPortTunnelable(buildEntry({ status: 'alreadyTunneled' }))).toBe(
			false
		);
		expect(isPortTunnelable(buildEntry({ status: 'ambiguous' }))).toBe(true);
	});
});
