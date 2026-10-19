import { describe, expect, it } from 'vitest';

import { buildExternalProcess } from '#/testing/factories';
import { buildImportDraft } from '#/externals/build_import_draft';

describe('buildImportDraft', () => {
	it('should take the host from the destination', () => {
		const process = buildExternalProcess();
		const [forward] = process.forwards;

		expect(buildImportDraft(process, forward).host).toBe('nova');
	});

	it('should leave the host empty without destination', () => {
		const process = buildExternalProcess({ destination: undefined });
		const [forward] = process.forwards;

		expect(buildImportDraft(process, forward).host).toBe('');
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
