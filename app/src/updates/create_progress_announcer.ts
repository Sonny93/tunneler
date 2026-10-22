import type { I18n } from '@lingui/core';

import type { UpdateProgressDto } from '#/daemon/protocol';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import {
	describeUpdateProgress,
	getDownloadStartText,
} from '#/updates/describe_update_progress';

export const DOWNLOAD_ANNOUNCEMENT_THROTTLE_MS = 5000;

type ProgressAnnouncer = {
	start: () => void;
	report: (progress: UpdateProgressDto) => void;
};

/**
 * Announces the installation politely: the start and the installation step
 * at once, the download progress at most once per throttle window.
 */
export function createProgressAnnouncer(
	i18n: I18n,
	now: () => number = Date.now
): ProgressAnnouncer {
	let lastDownloadAnnouncementAt = Number.NEGATIVE_INFINITY;
	const announce = (message: string) =>
		useAnnouncerStore.getState().announce(message, 'polite');

	return {
		start: () => announce(getDownloadStartText(i18n)),
		report: (progress) => {
			if (progress.kind === 'installing') {
				announce(describeUpdateProgress(progress, i18n).text);
				return;
			}
			const currentTime = now();
			if (
				currentTime - lastDownloadAnnouncementAt <
				DOWNLOAD_ANNOUNCEMENT_THROTTLE_MS
			)
				return;
			lastDownloadAnnouncementAt = currentTime;
			announce(describeUpdateProgress(progress, i18n).text);
		},
	};
}
