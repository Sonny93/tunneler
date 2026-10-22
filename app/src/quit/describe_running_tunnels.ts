import type { I18n } from '@lingui/core';
import { plural, t } from '@lingui/core/macro';

export function describeTunnelsToStop(count: number, i18n: I18n): string {
	return t(i18n)`${plural(count, {
		one: '# running tunnel or X11 session will be stopped.',
		other: '# running tunnels or X11 sessions will be stopped.',
	})}`;
}
