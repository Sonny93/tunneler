import { i18n } from '@lingui/core';
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
				'',
				i18n
			)
		).toBe('3333 to nova:3333');
	});

	it('should use the fallback host for a candidate without host', () => {
		expect(
			describeImportCandidateLabel(
				{ kind: 'local', listenPort: 3333 },
				'nova',
				i18n
			)
		).toBe('3333 to nova:3333');
	});

	it('should say the host is yet to choose without any host', () => {
		expect(
			describeImportCandidateLabel(
				{ kind: 'local', listenPort: 3333 },
				'',
				i18n
			)
		).toBe('3333 to host to choose:3333');
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
				null,
				i18n
			)
		).toBe('adonis · line 12');
	});

	it('should append the conflict note', () => {
		expect(
			describeImportCandidateDetails(
				{ host: 'nova', kind: 'local', listenPort: 3333 },
				'Already configured',
				i18n
			)
		).toBe('Already configured');
	});

	it('should be empty without comment, line or conflict', () => {
		expect(
			describeImportCandidateDetails(
				{ host: 'nova', kind: 'local', listenPort: 3333 },
				null,
				i18n
			)
		).toBe('');
	});
});
