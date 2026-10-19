import { describe, expect, it } from 'vitest';

import { describeImportConflict } from '#/import/describe_import_conflict';

describe('describeImportConflict', () => {
	it('should say the tunnel is already configured', () => {
		expect(describeImportConflict({ kind: 'alreadyConfigured' })).toBe(
			'Déjà configuré'
		);
	});

	it('should name the tunnel using the port', () => {
		expect(
			describeImportConflict({
				kind: 'listenConflict',
				tunnelLabel: '3333 → nova:3333',
			})
		).toBe('Port déjà utilisé par 3333 → nova:3333');
	});

	it('should point to an earlier candidate', () => {
		expect(describeImportConflict({ kind: 'duplicateInImport' })).toBe(
			"Même port qu'un tunnel plus haut"
		);
	});
});
