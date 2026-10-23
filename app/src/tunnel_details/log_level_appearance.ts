import type { MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { LogLevelDto } from '#/daemon/protocol';

type LogLevelAppearance = {
	icon: string;
	label: MessageDescriptor;
	toneClassName: string;
};

export const LOG_LEVEL_APPEARANCES: Readonly<
	Record<LogLevelDto, LogLevelAppearance>
> = {
	info: {
		icon: 'i-mdi-information-outline',
		label: msg`Info`,
		toneClassName: 'text-tn-info',
	},
	warn: {
		icon: 'i-mdi-alert-outline',
		label: msg`Warning`,
		toneClassName: 'text-tn-warning',
	},
	error: {
		icon: 'i-mdi-alert-octagon-outline',
		label: msg`Error`,
		toneClassName: 'text-tn-danger',
	},
};
