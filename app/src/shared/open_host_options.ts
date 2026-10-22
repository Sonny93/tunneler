import { msg } from '@lingui/core/macro';
import type { I18n, MessageDescriptor } from '@lingui/core';

import type { OpenHostDto } from '#/daemon/protocol';

type OpenHostOptionSpec = {
	value: OpenHostDto;
	label: MessageDescriptor;
	description: MessageDescriptor;
};

const OPEN_HOST_OPTION_SPECS: readonly OpenHostOptionSpec[] = [
	{
		value: 'localhost',
		label: msg`localhost`,
		description: msg`The localhost name`,
	},
	{
		value: 'ip',
		label: msg`IP address`,
		description: msg`127.0.0.1, or [::1] when the tunnel listens on IPv6`,
	},
];

export type OpenHostOption = {
	value: OpenHostDto;
	label: string;
	description: string;
};

/** The host choices of the "open" setting, worded in the active language. */
export function getOpenHostOptions(i18n: I18n): OpenHostOption[] {
	return OPEN_HOST_OPTION_SPECS.map(({ value, label, description }) => ({
		value,
		label: i18n._(label),
		description: i18n._(description),
	}));
}

export function describeOpenHost(host: OpenHostDto, i18n: I18n): string {
	const spec = OPEN_HOST_OPTION_SPECS.find(({ value }) => value === host);
	return spec === undefined ? host : i18n._(spec.label);
}
