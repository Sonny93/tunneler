import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeUpdateProgress } from '#/updates/describe_update_progress';

const MEGABYTE = 1024 * 1024;

describe('describeUpdateProgress', () => {
	it('should say the download starts before any progress is known', () => {
		expect(describeUpdateProgress(null, i18n)).toEqual({
			percent: null,
			text: 'Downloading the update…',
		});
	});

	it('should give the percentage and the sizes when the total is known', () => {
		expect(
			describeUpdateProgress(
				{
					kind: 'downloading',
					downloadedBytes: MEGABYTE,
					totalBytes: 4 * MEGABYTE,
				},
				i18n
			)
		).toEqual({
			percent: 25,
			text: 'Downloading: 25% (1 MB of 4 MB)',
		});
	});

	it('should give only the downloaded size when the total is unknown', () => {
		expect(
			describeUpdateProgress(
				{ kind: 'downloading', downloadedBytes: 2 * MEGABYTE },
				i18n
			)
		).toEqual({ percent: null, text: 'Downloading: 2 MB' });
	});

	it('should never exceed 100 percent', () => {
		expect(
			describeUpdateProgress(
				{
					kind: 'downloading',
					downloadedBytes: 2 * MEGABYTE,
					totalBytes: MEGABYTE,
				},
				i18n
			).percent
		).toBe(100);
	});

	it('should say the installation runs once the download is over', () => {
		expect(describeUpdateProgress({ kind: 'installing' }, i18n)).toEqual({
			percent: null,
			text: 'Installing, Tunneler will restart.',
		});
	});
});
