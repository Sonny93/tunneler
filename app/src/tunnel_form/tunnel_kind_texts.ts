import type { MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { TunnelKindDto } from '#/daemon/protocol';

export const TUNNEL_KIND_OPTIONS = [
	{
		value: 'local',
		label: msg`Local (-L)`,
		description: msg`Listens on this computer, target reached from the server`,
	},
	{
		value: 'remote',
		label: msg`Remote (-R)`,
		description: msg`Listens on the server, target reached from this computer`,
	},
	{
		value: 'dynamic',
		label: msg`Dynamic (-D, SOCKS)`,
		description: msg`SOCKS proxy on this computer`,
	},
] as const satisfies readonly {
	value: TunnelKindDto;
	label: MessageDescriptor;
	description: MessageDescriptor;
}[];

type TunnelKindFieldTexts = {
	listenPortLabel: MessageDescriptor;
	targetHostLabel: MessageDescriptor | null;
	targetHostHint: MessageDescriptor | null;
};

const FIELD_TEXTS: Readonly<Record<TunnelKindDto, TunnelKindFieldTexts>> = {
	local: {
		listenPortLabel: msg`Listen port`,
		targetHostLabel: msg`Target host`,
		targetHostHint: msg`Seen from the SSH server`,
	},
	remote: {
		listenPortLabel: msg`Listen port on the server`,
		targetHostLabel: msg`Target host (from this computer)`,
		targetHostHint: msg`Reached from this computer`,
	},
	dynamic: {
		listenPortLabel: msg`Listen port`,
		targetHostLabel: null,
		targetHostHint: null,
	},
};

export function getTunnelKindFieldTexts(
	kind: TunnelKindDto
): TunnelKindFieldTexts {
	return FIELD_TEXTS[kind];
}
