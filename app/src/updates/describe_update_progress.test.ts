import { describe, expect, it } from 'vitest';

import { describeUpdateProgress } from '#/updates/describe_update_progress';

const MEGABYTE = 1024 * 1024;

describe('describeUpdateProgress', () => {
	it('should say the download starts before any progress is known', () => {
		expect(describeUpdateProgress(null)).toEqual({
			percent: null,
			text: 'Téléchargement de la mise à jour…',
		});
	});

	it('should give the percentage and the sizes when the total is known', () => {
		expect(
			describeUpdateProgress({
				kind: 'downloading',
				downloadedBytes: MEGABYTE,
				totalBytes: 4 * MEGABYTE,
			})
		).toEqual({
			percent: 25,
			text: 'Téléchargement : 25 % (1 Mo sur 4 Mo)',
		});
	});

	it('should give only the downloaded size when the total is unknown', () => {
		expect(
			describeUpdateProgress({
				kind: 'downloading',
				downloadedBytes: 2 * MEGABYTE,
			})
		).toEqual({ percent: null, text: 'Téléchargement : 2 Mo' });
	});

	it('should never exceed 100 percent', () => {
		expect(
			describeUpdateProgress({
				kind: 'downloading',
				downloadedBytes: 2 * MEGABYTE,
				totalBytes: MEGABYTE,
			}).percent
		).toBe(100);
	});

	it('should say the installation runs once the download is over', () => {
		expect(describeUpdateProgress({ kind: 'installing' })).toEqual({
			percent: null,
			text: 'Installation en cours, Tunneler va redémarrer.',
		});
	});
});
