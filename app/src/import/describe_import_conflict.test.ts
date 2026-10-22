import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeImportConflict } from '#/import/describe_import_conflict';

describe('describeImportConflict', () => {
	it('should say the tunnel is already configured', () => {
		expect(describeImportConflict({ kind: 'alreadyConfigured' }, i18n)).toBe(
			'Already configured'
		);
	});

	it('should name the tunnel using the port', () => {
		expect(
			describeImportConflict(
				{ kind: 'listenConflict', tunnelLabel: '3333 → nova:3333' },
				i18n
			)
		).toBe('Port already used by 3333 → nova:3333');
	});

	it('should point to an earlier candidate', () => {
		expect(describeImportConflict({ kind: 'duplicateInImport' }, i18n)).toBe(
			'Same port as a tunnel above'
		);
	});
});
