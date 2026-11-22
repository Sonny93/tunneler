import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeImportConflict } from '#/import/describe_import_conflict';
import { buildTunnel } from '#/testing/factories';

describe('describeImportConflict', () => {
	it('should say the tunnel is already configured', () => {
		expect(
			describeImportConflict({ kind: 'alreadyConfigured' }, [], i18n)
		).toBe('Already configured');
	});

	it('should name the tunnel using the port', () => {
		expect(
			describeImportConflict(
				{ kind: 'listenConflict', holderId: 'holder' },
				[buildTunnel({ id: 'holder', listenPort: 3334, targetPort: 80 })],
				i18n
			)
		).toBe('Port already used by 3334 → nova:80');
	});

	it('should say another tunnel when the holder is no longer known', () => {
		expect(
			describeImportConflict(
				{ kind: 'listenConflict', holderId: 'gone' },
				[],
				i18n
			)
		).toBe('Port already used by another tunnel');
	});

	it('should point to an earlier candidate', () => {
		expect(
			describeImportConflict({ kind: 'duplicateInImport' }, [], i18n)
		).toBe('Same port as a tunnel above');
	});
});
