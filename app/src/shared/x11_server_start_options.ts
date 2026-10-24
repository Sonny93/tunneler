import type { I18n, MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { X11ServerStartDto } from '#/daemon/protocol';

type X11ServerStartOptionSpec = {
	value: X11ServerStartDto;
	label: MessageDescriptor;
};

const X11_SERVER_START_OPTION_SPECS: readonly X11ServerStartOptionSpec[] = [
	{ value: 'ask', label: msg`Ask me` },
	{ value: 'always', label: msg`Start it` },
	{ value: 'never', label: msg`Don't start it` },
];

export type X11ServerStartOption = {
	value: X11ServerStartDto;
	label: string;
};

/** The choices of the "when no X server is running" setting, worded in the active language. */
export function getX11ServerStartOptions(i18n: I18n): X11ServerStartOption[] {
	return X11_SERVER_START_OPTION_SPECS.map(({ value, label }) => ({
		value,
		label: i18n._(label),
	}));
}
