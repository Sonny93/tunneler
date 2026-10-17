import { describe, expect, it } from 'vitest';

import { describeImportConflict } from '#/import/describe_import_conflict';

describe('describeImportConflict', () => {
	it('should name the tunnel already configured', () => {
		expect(
			describeImportConflict({
				kind: 'alreadyConfigured',
				tunnelName: 'Adonis',
			})
		).toBe('Déjà configuré (Adonis)');
	});

	it('should name the tunnel using the port', () => {
		expect(
			describeImportConflict({ kind: 'listenConflict', tunnelName: 'Adonis' })
		).toBe('Port déjà utilisé par Adonis');
	});

	it('should point to an earlier candidate', () => {
		expect(describeImportConflict({ kind: 'duplicateInImport' })).toBe(
			"Même port qu'un tunnel plus haut"
		);
	});
});
