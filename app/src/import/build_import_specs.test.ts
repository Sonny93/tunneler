import { describe, expect, it } from 'vitest';

import { buildImportSpecs } from '#/import/build_import_specs';

const BASE_INPUT = {
	selectedIndexes: new Set([0, 1]),
	fallbackHost: 'fallback',
	tags: ['imported'],
};

describe('buildImportSpecs', () => {
	it('should build a spec per selected candidate with the tags', () => {
		expect(
			buildImportSpecs({
				...BASE_INPUT,
				candidates: [
					{ host: 'nova', kind: 'local', listenPort: 3333 },
					{ host: 'nova', kind: 'local', listenPort: 4444 },
				],
			}).map(({ spec }) => [spec.listenPort, spec.tags])
		).toEqual([
			[3333, ['imported']],
			[4444, ['imported']],
		]);
	});

	it('should skip candidates that are not selected', () => {
		expect(
			buildImportSpecs({
				...BASE_INPUT,
				selectedIndexes: new Set([1]),
				candidates: [
					{ host: 'nova', kind: 'local', listenPort: 3333 },
					{ host: 'nova', kind: 'local', listenPort: 4444 },
				],
			}).map(({ index }) => index)
		).toEqual([1]);
	});

	it('should use the fallback host for a candidate without host', () => {
		const [first] = buildImportSpecs({
			...BASE_INPUT,
			candidates: [{ kind: 'local', listenPort: 3333 }],
		});

		expect(first?.spec.host).toBe('fallback');
	});

	it('should leave out the target port when it equals the listen port', () => {
		const [first] = buildImportSpecs({
			...BASE_INPUT,
			candidates: [
				{ host: 'nova', kind: 'local', listenPort: 3333, targetPort: 3333 },
			],
		});

		expect(first?.spec).not.toHaveProperty('targetPort');
	});

	it('should keep a different target port, target host and listen address', () => {
		const [first] = buildImportSpecs({
			...BASE_INPUT,
			candidates: [
				{
					host: 'nova',
					kind: 'remote',
					listenPort: 8080,
					listenAddress: '0.0.0.0',
					targetHost: 'db',
					targetPort: 5432,
				},
			],
		});

		expect(first?.spec).toEqual({
			host: 'nova',
			kind: 'remote',
			listenPort: 8080,
			listenAddress: '0.0.0.0',
			targetHost: 'db',
			targetPort: 5432,
			tags: ['imported'],
		});
	});

	it('should not give a dynamic forward a target', () => {
		const [first] = buildImportSpecs({
			...BASE_INPUT,
			candidates: [
				{
					host: 'nova',
					kind: 'dynamic',
					listenPort: 1080,
					targetHost: 'x',
					targetPort: 1,
				},
			],
		});

		expect(first?.spec).toEqual({
			host: 'nova',
			kind: 'dynamic',
			listenPort: 1080,
			tags: ['imported'],
		});
	});
});
