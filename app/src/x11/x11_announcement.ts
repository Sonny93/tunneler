import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { X11StateDto } from '#/daemon/protocol';
import { getX11StateDetail, getX11StateLabel } from '#/x11/x11_state_label';

/** "X11 nova: active (localhost:10.0)". */
export function buildX11Announcement(
	host: string,
	state: Readonly<X11StateDto>,
	i18n: I18n
): string {
	const label = getX11StateLabel(state, i18n).toLowerCase();
	const detail = getX11StateDetail(state);
	if (detail === null) return i18n._(msg`X11 ${host}: ${label}`);
	return i18n._(msg`X11 ${host}: ${label} (${detail})`);
}
