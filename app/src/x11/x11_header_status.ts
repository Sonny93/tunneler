import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { X11StateDto } from '#/daemon/protocol';
import { getX11StateLabel } from '#/x11/x11_state_label';

/** The short status shown in a host header, or `null` while the anchor is stopped. */
export function getX11HeaderStatus(
	state: Readonly<X11StateDto>,
	i18n: I18n
): string | null {
	if (state.kind === 'stopped') return null;
	if (state.kind === 'active') {
		const { display } = state;
		return t(i18n)`X11 active: ${display}`;
	}
	const label = getX11StateLabel(state, i18n).toLowerCase();
	return t(i18n)`X11: ${label}`;
}
