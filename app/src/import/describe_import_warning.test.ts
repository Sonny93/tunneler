import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeImportWarning } from '#/import/describe_import_warning';

const describeWarning = (
	warning: Parameters<typeof describeImportWarning>[0]
) => describeImportWarning(warning, i18n);

describe('describeImportWarning', () => {
	it('should describe a malformed forward with its line', () => {
		expect(
			describeWarning({
				kind: 'malformedForward',
				line: 4,
				text: '-L nope',
			})
		).toEqual({
			before: 'Line 4: option ignored, unreadable: ',
			raw: '-L nope',
			after: '',
		});
	});

	it('should describe an unsupported forward with line and host', () => {
		expect(
			describeWarning({
				kind: 'unsupportedForward',
				line: 7,
				host: 'nova',
				text: '-L /tmp/a.sock:x:1',
			})
		).toEqual({
			before:
				'Line 7: nova: unsupported forward (Unix socket or remote SOCKS): ',
			raw: '-L /tmp/a.sock:x:1',
			after: '',
		});
	});

	it('should leave out the line and host of an unsupported forward when absent', () => {
		expect(
			describeWarning({ kind: 'unsupportedForward', text: '-R 1080' })
		).toEqual({
			before: 'Unsupported forward (Unix socket or remote SOCKS): ',
			raw: '-R 1080',
			after: '',
		});
	});

	it('should describe an unknown destination', () => {
		expect(
			describeWarning({
				kind: 'unknownDestination',
				destination: 'user@198.51.100.10',
			})
		).toEqual({
			before: 'No ssh_config alias matches ',
			raw: 'user@198.51.100.10',
			after: ': choose the host below.',
		});
	});

	it('should describe a host resolution failure', () => {
		expect(
			describeWarning({
				kind: 'hostResolutionFailed',
				host: 'nova',
				message: 'bad config',
			})
		).toEqual({
			before: "Couldn't read nova: ",
			raw: 'bad config',
			after: '',
		});
	});
});
