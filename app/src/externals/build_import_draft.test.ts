import { describe, expect, it } from 'vitest';

import { buildImportDraft } from '#/externals/build_import_draft';
import { buildExternalProcess } from '#/testing/factories';

describe('buildImportDraft', () => {
	it('should take the host from the destination and link the target port when it equals the listen port', () => {
		const process = buildExternalProcess();
		const [forward] = process.forwards;

		const draft = buildImportDraft(process, forward);

		expect(draft.host).toBe('nova');
		expect(draft.targetPort).toBeUndefined();
	});

	it('should leave the host empty without destination', () => {
		const process = buildExternalProcess({ destination: undefined });
		const [forward] = process.forwards;

		expect(buildImportDraft(process, forward).host).toBe('');
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
