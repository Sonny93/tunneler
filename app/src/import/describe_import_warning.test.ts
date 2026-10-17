import { describe, expect, it } from 'vitest';

import { describeImportWarning } from '#/import/describe_import_warning';

describe('describeImportWarning', () => {
	it('should describe a malformed forward with its line', () => {
		expect(
			describeImportWarning({
				kind: 'malformedForward',
				line: 4,
				text: '-L nope',
			})
		).toEqual({
			before: 'Ligne 4 : option ignorée, illisible : ',
			raw: '-L nope',
			after: '',
		});
	});

	it('should describe an unsupported forward with line and host', () => {
		expect(
			describeImportWarning({
				kind: 'unsupportedForward',
				line: 7,
				host: 'nova',
				text: '-L /tmp/a.sock:x:1',
			})
		).toEqual({
			before:
				'Ligne 7 : nova : forward non pris en charge (socket Unix ou SOCKS distant) : ',
			raw: '-L /tmp/a.sock:x:1',
			after: '',
		});
	});

	it('should leave out the line and host of an unsupported forward when absent', () => {
		expect(
			describeImportWarning({ kind: 'unsupportedForward', text: '-R 1080' })
		).toEqual({
			before: 'forward non pris en charge (socket Unix ou SOCKS distant) : ',
			raw: '-R 1080',
			after: '',
		});
	});

	it('should describe an unknown destination', () => {
		expect(
			describeImportWarning({
				kind: 'unknownDestination',
				destination: 'user@198.51.100.10',
			})
		).toEqual({
			before: 'Aucun alias ssh_config ne correspond à ',
			raw: 'user@198.51.100.10',
			after: " : choisissez l'hôte ci-dessous.",
		});
	});

	it('should describe a host resolution failure', () => {
		expect(
			describeImportWarning({
				kind: 'hostResolutionFailed',
				host: 'nova',
				message: 'bad config',
			})
		).toEqual({
			before: 'Lecture de nova impossible : ',
			raw: 'bad config',
			after: '',
		});
	});
});
