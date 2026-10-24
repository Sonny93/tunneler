import type { I18n, MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { OpenDefaultDto } from '#/daemon/protocol';

type OpenDefaultOptionSpec = {
	value: OpenDefaultDto;
	label: MessageDescriptor;
	description: MessageDescriptor;
};

const OPEN_DEFAULT_OPTION_SPECS: readonly OpenDefaultOptionSpec[] = [
	{
		value: 'suggested',
		label: msg`Suggested application`,
		description: msg`The application listed for the target port, else the browser`,
	},
	{
		value: 'browser',
		label: msg`Always the browser`,
		description: msg`Applications are opened only when the tunnel or the command asks for one`,
	},
];

export type OpenDefaultOption = {
	value: OpenDefaultDto;
	label: string;
	description: string;
};

/** The choices of the "default opening" setting, worded in the active language. */
export function getOpenDefaultOptions(i18n: I18n): OpenDefaultOption[] {
	return OPEN_DEFAULT_OPTION_SPECS.map(({ value, label, description }) => ({
		value,
		label: i18n._(label),
		description: i18n._(description),
	}));
}
