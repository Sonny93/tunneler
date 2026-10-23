import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { UpdateProgressDto } from '#/daemon/protocol';
import { formatBytes } from '#/shared/format_bytes';

const PERCENT_SCALE = 100;

export type UpdateProgressView = {
	/** `null` when the amount of work is unknown. */
	percent: number | null;
	text: string;
};

export function getDownloadStartText(i18n: I18n): string {
	return i18n._(msg`Downloading the update…`);
}

export function describeUpdateProgress(
	progress: Readonly<UpdateProgressDto> | null,
	i18n: I18n
): UpdateProgressView {
	if (progress === null) {
		return { percent: null, text: getDownloadStartText(i18n) };
	}
	if (progress.kind === 'installing') {
		return {
			percent: null,
			text: i18n._(msg`Installing, Tunneler will restart.`),
		};
	}
	const { downloadedBytes, totalBytes } = progress;
	const downloaded = formatBytes(downloadedBytes);
	if (totalBytes === undefined || totalBytes === 0) {
		return { percent: null, text: i18n._(msg`Downloading: ${downloaded}`) };
	}
	const percent = Math.min(
		PERCENT_SCALE,
		Math.floor((downloadedBytes / totalBytes) * PERCENT_SCALE)
	);
	const total = formatBytes(totalBytes);
	return {
		percent,
		text: i18n._(msg`Downloading: ${percent}% (${downloaded} of ${total})`),
	};
}
