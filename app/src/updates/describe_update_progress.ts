import { formatBytes } from '#/shared/format_bytes';
import type { UpdateProgressDto } from '#/daemon/protocol';

const PERCENT_SCALE = 100;

export type UpdateProgressView = {
	/** `null` when the amount of work is unknown. */
	percent: number | null;
	text: string;
};

export const INSTALLING_TEXT = 'Installation en cours, Tunneler va redémarrer.';
export const DOWNLOAD_START_TEXT = 'Téléchargement de la mise à jour…';

export function describeUpdateProgress(
	progress: Readonly<UpdateProgressDto> | null
): UpdateProgressView {
	if (progress === null) return { percent: null, text: DOWNLOAD_START_TEXT };
	if (progress.kind === 'installing')
		return { percent: null, text: INSTALLING_TEXT };
	const { downloadedBytes, totalBytes } = progress;
	if (totalBytes === undefined || totalBytes === 0)
		return {
			percent: null,
			text: `Téléchargement : ${formatBytes(downloadedBytes)}`,
		};
	const percent = Math.min(
		PERCENT_SCALE,
		Math.floor((downloadedBytes / totalBytes) * PERCENT_SCALE)
	);
	return {
		percent,
		text: `Téléchargement : ${percent} % (${formatBytes(downloadedBytes)} sur ${formatBytes(totalBytes)})`,
	};
}
