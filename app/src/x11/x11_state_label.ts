import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { X11StateDto } from '#/daemon/protocol';

export function getX11StateLabel(
	state: Readonly<X11StateDto>,
	i18n: I18n
): string {
	switch (state.kind) {
		case 'stopped':
			return i18n._(msg`Stopped`);
		case 'starting':
			return i18n._(msg`Starting…`);
		case 'active':
			return i18n._(msg`Active`);
		case 'reconnecting': {
			const { attempt } = state;
			return i18n._(msg`Reconnecting (attempt ${attempt})`);
		}
		case 'failed':
			return i18n._(msg`Failed`);
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
