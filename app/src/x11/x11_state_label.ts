import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type { X11StateDto } from '#/daemon/protocol';

export function getX11StateLabel(
	state: Readonly<X11StateDto>,
	i18n: I18n
): string {
	switch (state.kind) {
		case 'stopped':
			return t(i18n)`Stopped`;
		case 'starting':
			return t(i18n)`Starting…`;
		case 'active':
			return t(i18n)`Active`;
		case 'reconnecting': {
			const { attempt } = state;
			return t(i18n)`Reconnecting (attempt ${attempt})`;
		}
		case 'failed':
			return t(i18n)`Failed`;
	}
}

/** The remote display of an active anchor or the message of a failed one. */
export function getX11StateDetail(state: Readonly<X11StateDto>): string | null {
	switch (state.kind) {
		case 'active':
			return state.display;
		case 'failed':
			return state.message;
		case 'stopped':
		case 'starting':
		case 'reconnecting':
			return null;
	}
}
