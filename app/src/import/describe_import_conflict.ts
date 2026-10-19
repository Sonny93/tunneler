import type { ImportConflict } from '#/import/find_import_conflicts';

export function describeImportConflict(
	conflict: Readonly<ImportConflict>
): string {
	switch (conflict.kind) {
		case 'alreadyConfigured':
			return 'Déjà configuré';
		case 'listenConflict':
			return `Port déjà utilisé par ${conflict.tunnelLabel}`;
		case 'duplicateInImport':
			return "Même port qu'un tunnel plus haut";
	}
}
