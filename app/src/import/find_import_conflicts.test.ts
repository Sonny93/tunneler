import { describe, expect, it } from 'vitest';

import { findImportConflicts } from '#/import/find_import_conflicts';
import { buildSettings, buildTunnel } from '#/testing/factories';

const SETTINGS = buildSettings();

function findFirst(
	candidate: Parameters<typeof findImportConflicts>[0][number],
	existing: Parameters<typeof findImportConflicts>[1] = [],
	fallbackHost: string | null = null
) {
	return findImportConflicts([candidate], existing, SETTINGS, fallbackHost)[0];
}

describe('findImportConflicts', () => {
	it('should report nothing without any existing tunnel', () => {
		expect(
			findFirst({ host: 'nova', kind: 'local', listenPort: 3333 })
		).toBeNull();
	});

	it('should flag an identical tunnel as already configured', () => {
		expect(
			findFirst({ host: 'nova', kind: 'local', listenPort: 3333 }, [
				buildTunnel(),
			])
		).toEqual({ kind: 'alreadyConfigured' });
	});

	it('should compare the effective target', () => {
		expect(
			findFirst(
				{
					host: 'nova',
					kind: 'local',
					listenPort: 3333,
					targetHost: '127.0.0.1',
					targetPort: 3333,
				},
				[buildTunnel()]
			)
		).toEqual({ kind: 'alreadyConfigured' });
	});

	it('should flag the same port with another target as a listen conflict', () => {
		expect(
			findFirst(
				{ host: 'nova', kind: 'local', listenPort: 3333, targetPort: 80 },
				[buildTunnel()]
			)
		).toEqual({ kind: 'listenConflict', tunnelLabel: '3333 → nova:3333' });
	});

	it('should flag a listen conflict whatever the ssh host', () => {
		expect(
			findFirst({ host: 'other', kind: 'local', listenPort: 3333 }, [
				buildTunnel(),
			])
		).toEqual({ kind: 'listenConflict', tunnelLabel: '3333 → nova:3333' });
	});

	it('should treat local and dynamic forwards as sharing the local ports', () => {
		expect(
			findFirst({ host: 'nova', kind: 'dynamic', listenPort: 3333 }, [
				buildTunnel(),
			])
		).toEqual({ kind: 'listenConflict', tunnelLabel: '3333 → nova:3333' });
	});

	it('should not flag another listen address', () => {
		expect(
			findFirst(
				{
					host: 'nova',
					kind: 'local',
					listenPort: 3333,
					listenAddress: '127.0.0.2',
				},
				[buildTunnel()]
			)
		).toBeNull();
	});

	it('should not conflict a remote forward with a local one', () => {
		expect(
			findFirst({ host: 'nova', kind: 'remote', listenPort: 3333 }, [
				buildTunnel(),
			])
		).toBeNull();
	});

	it('should flag a remote forward on the same host, address and port', () => {
		expect(
			findFirst(
				{ host: 'nova', kind: 'remote', listenPort: 8080, targetPort: 80 },
				[buildTunnel({ kind: 'remote', listenPort: 8080 })]
			)
		).toEqual({
			kind: 'listenConflict',
			tunnelLabel: 'nova:8080 → 127.0.0.1:8080',
		});
	});

	it('should not flag a remote forward on another host', () => {
		expect(
			findFirst({ host: 'other', kind: 'remote', listenPort: 8080 }, [
				buildTunnel({ kind: 'remote', listenPort: 8080 }),
			])
		).toBeNull();
	});

	it('should use the fallback host for a candidate without host', () => {
		expect(
			findFirst({ kind: 'local', listenPort: 3333 }, [buildTunnel()], 'nova')
		).toEqual({ kind: 'alreadyConfigured' });
	});

	it('should flag a later candidate repeating an earlier listen identity', () => {
		expect(
			findImportConflicts(
				[
					{ host: 'nova', kind: 'local', listenPort: 4000 },
					{ host: 'nova', kind: 'local', listenPort: 4000 },
				],
				[],
				SETTINGS,
				null
			)
		).toEqual([null, { kind: 'duplicateInImport' }]);
	});
});
