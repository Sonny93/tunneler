import { describe, expect, it } from 'vitest';

import { buildExternalProcess } from '#/testing/factories';
import { buildImportDraft } from '#/externals/build_import_draft';

describe('buildImportDraft', () => {
	it('should suggest a name from the destination and the listen port', () => {
		const process = buildExternalProcess();
		const [forward] = process.forwards;

		expect(buildImportDraft(process, forward).name).toBe('nova 3333');
	});

	it('should use ssh as name prefix and an empty host without destination', () => {
		const process = buildExternalProcess({ destination: undefined });
		const [forward] = process.forwards;

		expect(buildImportDraft(process, forward)).toMatchObject({
			name: 'ssh 3333',
			host: '',
		});
	});

	it('should link the target port when it equals the listen port', () => {
		const process = buildExternalProcess();
		const [forward] = process.forwards;

		expect(buildImportDraft(process, forward).targetPort).toBeUndefined();
	});

	it('should keep a target port that differs from the listen port', () => {
		const process = buildExternalProcess({
			forwards: [
				{ kind: 'local', listenPort: 3333, targetHost: 'db', targetPort: 5432 },
			],
		});
		const [forward] = process.forwards;

		expect(buildImportDraft(process, forward)).toMatchObject({
			kind: 'local',
			targetHost: 'db',
			targetPort: 5432,
			tags: [],
		});
	});
});
