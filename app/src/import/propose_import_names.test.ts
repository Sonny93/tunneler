import { describe, expect, it } from 'vitest';

import { proposeImportNames } from '#/import/propose_import_names';

describe('proposeImportNames', () => {
	it('should prefer the label', () => {
		expect(
			proposeImportNames(
				[{ label: 'Adonis', host: 'nova', listenPort: 3333 }],
				[]
			)
		).toEqual(['Adonis']);
	});

	it('should combine the host and the listen port without a label', () => {
		expect(
			proposeImportNames([{ host: 'nova', listenPort: 3333 }], [])
		).toEqual(['nova 3333']);
	});

	it('should fall back to ssh without a host', () => {
		expect(proposeImportNames([{ listenPort: 3333 }], [])).toEqual([
			'ssh 3333',
		]);
	});

	it('should suffix a name taken by an existing tunnel, ignoring case', () => {
		expect(
			proposeImportNames([{ host: 'nova', listenPort: 3333 }], ['NOVA 3333'])
		).toEqual(['nova 3333 2']);
	});

	it('should suffix repeated names within the import', () => {
		expect(
			proposeImportNames(
				[
					{ host: 'nova', listenPort: 3333 },
					{ host: 'nova', listenPort: 3333 },
					{ host: 'NOVA', listenPort: 3333 },
				],
				[]
			)
		).toEqual(['nova 3333', 'nova 3333 2', 'NOVA 3333 3']);
	});
});
