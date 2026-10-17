import { describe, expect, it } from 'vitest';

import { describeImportCandidate } from '#/import/describe_import_candidate';

describe('describeImportCandidate', () => {
	it('should give the endpoint and the source line', () => {
		expect(
			describeImportCandidate(
				{ host: 'nova', kind: 'local', listenPort: 3333, line: 12 },
				'',
				null
			)
		).toBe('3333 vers nova:3333 · ligne 12');
	});

	it('should append the conflict note', () => {
		expect(
			describeImportCandidate(
				{ host: 'nova', kind: 'local', listenPort: 3333 },
				'',
				'Déjà configuré (Adonis)'
			)
		).toBe('3333 vers nova:3333 · Déjà configuré (Adonis)');
	});

	it('should use the fallback host for a candidate without host', () => {
		expect(
			describeImportCandidate({ kind: 'local', listenPort: 3333 }, 'nova', null)
		).toBe('3333 vers nova:3333');
	});

	it('should say the host is yet to choose without any host', () => {
		expect(
			describeImportCandidate({ kind: 'local', listenPort: 3333 }, '', null)
		).toBe('3333 vers hôte à choisir:3333');
	});
});
