import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { TrayStatusDto } from '#/daemon/protocol';
import type { TrayProblemKind } from '#/state/ui_store';

export type TrayProblem = Extract<TrayStatusDto, { kind: TrayProblemKind }>;

export type TrayStatusNoticeText = { heading: string; body: string };

export function isTrayProblem(
	status: Readonly<TrayStatusDto>
): status is TrayProblem {
	return status.kind === 'missingHost' || status.kind === 'failed';
}

export function describeTrayProblem(
	problem: Readonly<TrayProblem>,
	i18n: I18n
): TrayStatusNoticeText {
	if (problem.kind === 'missingHost') {
		return {
			heading: t(i18n)`No notification area detected`,
			body: t(
				i18n
			)`Tunneler keeps running when you close this window. To reopen it, run “tunneler” again. On GNOME, install the “AppIndicator and KStatusNotifierItem Support” extension to show the icon.`,
		};
	}
	const { message } = problem;
	return {
		heading: t(i18n)`Notification icon unavailable`,
		body: t(
			i18n
		)`The icon could not be created: ${message}. Tunneler keeps running when you close this window; run “tunneler” again to reopen it.`,
	};
}
