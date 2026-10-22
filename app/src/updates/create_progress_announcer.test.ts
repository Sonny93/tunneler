import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import {
	createProgressAnnouncer,
	DOWNLOAD_ANNOUNCEMENT_THROTTLE_MS,
} from '#/updates/create_progress_announcer';

const downloading = (downloadedBytes: number) =>
	({ kind: 'downloading', downloadedBytes, totalBytes: 200 }) as const;

const politeMessage = () => useAnnouncerStore.getState().polite.message;

describe('createProgressAnnouncer', () => {
	it('should announce the start of the download', () => {
		createProgressAnnouncer(i18n).start();

		expect(politeMessage()).toBe('Downloading the update…');
	});

	it('should announce the first download progress at once', () => {
		createProgressAnnouncer(i18n, () => 0).report(downloading(20));

		expect(politeMessage()).toBe('Downloading: 10% (20 B of 200 B)');
	});

	it('should not announce a download progress inside the throttle window', () => {
		let currentTime = 0;
		const announcer = createProgressAnnouncer(i18n, () => currentTime);
		announcer.report(downloading(20));
		currentTime = DOWNLOAD_ANNOUNCEMENT_THROTTLE_MS - 1;

		announcer.report(downloading(100));

		expect(politeMessage()).toContain('10%');
	});

	it('should announce a download progress once the throttle window is over', () => {
		let currentTime = 0;
		const announcer = createProgressAnnouncer(i18n, () => currentTime);
		announcer.report(downloading(20));
		currentTime = DOWNLOAD_ANNOUNCEMENT_THROTTLE_MS;

		announcer.report(downloading(100));

		expect(politeMessage()).toContain('50%');
	});

	it('should announce the installation step even inside the throttle window', () => {
		const announcer = createProgressAnnouncer(i18n, () => 0);
		announcer.report(downloading(20));

		announcer.report({ kind: 'installing' });

		expect(politeMessage()).toBe('Installing, Tunneler will restart.');
	});
});
