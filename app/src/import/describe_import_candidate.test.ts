import { describe, expect, it } from 'vitest';

import {
	describeImportCandidateDetails,
	describeImportCandidateLabel,
} from '#/import/describe_import_candidate';

describe('describeImportCandidateLabel', () => {
	it('should give the endpoint in words', () => {
		expect(
			describeImportCandidateLabel(
				{ host: 'nova', kind: 'local', listenPort: 3333 },
				''
			)
		).toBe('3333 vers nova:3333');
	});

	it('should use the fallback host for a candidate without host', () => {
		expect(
			describeImportCandidateLabel({ kind: 'local', listenPort: 3333 }, 'nova')
		).toBe('3333 vers nova:3333');
	});

	it('should say the host is yet to choose without any host', () => {
		expect(
			describeImportCandidateLabel({ kind: 'local', listenPort: 3333 }, '')
		).toBe('3333 vers hôte à choisir:3333');
	});
});

describe('describeImportCandidateDetails', () => {
	it('should give the comment and the source line', () => {
		expect(
			describeImportCandidateDetails(
				{
					host: 'nova',
					kind: 'local',
					listenPort: 3333,
					comment: 'adonis',
					line: 12,
				},
				null
			)
		).toBe('adonis · ligne 12');
	});

	it('should append the conflict note', () => {
		expect(
			describeImportCandidateDetails(
				{ host: 'nova', kind: 'local', listenPort: 3333 },
				'Déjà configuré'
			)
		).toBe('Déjà configuré');
	});

	it('should be empty without comment, line or conflict', () => {
		expect(
			describeImportCandidateDetails(
				{ host: 'nova', kind: 'local', listenPort: 3333 },
				null
			)
		).toBe('');
	});
});
