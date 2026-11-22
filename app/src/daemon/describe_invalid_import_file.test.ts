import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';

const describeData = (data: unknown): string =>
	describeDaemonError(
		new DaemonCommandError('invalid_import_file', 'raw text', data),
		i18n
	);

describe('describeDaemonError for invalid_import_file', () => {
	it.each([
		[
			{ reason: 'syntax', line: 4, message: 'expected `=`' },
			'The file cannot be read (line 4): expected `=`',
		],
		[{ reason: 'syntax' }, 'The file cannot be read.'],
		[
			{ reason: 'notTunnelerFile' },
			'This is not a Tunneler export or tunnels file.',
		],
		[
			{ reason: 'newerVersion', found: 3, supported: 1 },
			'This file was written by a newer version of Tunneler (format 3, this version reads up to 1).',
		],
		[
			{ reason: 'unsafeCharacters', entry: 0, field: 'host' },
			'Tunnel 1 of the file has control or hidden characters in the field host, so the whole file was refused.',
		],
		[
			{ reason: 'unsafeCharacters', entry: 2, field: 'open.host_name' },
			'Tunnel 3 of the file has control or hidden characters in the field open host name, so the whole file was refused.',
		],
		[
			{ reason: 'unsafeCharacters', entry: 0, field: 'open.future' },
			'Tunnel 1 of the file has control or hidden characters in the field open.future, so the whole file was refused.',
		],
		[
			{ reason: 'tooLarge', limit: 262_144 },
			'The file is larger than 256 KiB.',
		],
		[
			{ reason: 'tooLarge' },
			"This file's plan is too large to show: import a file with fewer tunnels",
		],
		[
			{ reason: 'tooManyEntries', limit: 1000 },
			'The file has more than 1000 tunnels.',
		],
	])('should describe %j', (data, expected) => {
		expect(describeData(data)).toBe(expected);
	});

	it('should fall back to the raw message when the data is unusable', () => {
		expect(describeData(undefined)).toBe('raw text');
		expect(describeData({ reason: 'unknown' })).toBe('raw text');
	});
});
