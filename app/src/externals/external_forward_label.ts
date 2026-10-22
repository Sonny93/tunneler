import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { ExternalForwardDto, TunnelKindDto } from '#/daemon/protocol';
import {
	SPOKEN_SEPARATOR,
	VISUAL_SEPARATOR,
} from '#/tunnels/tunnel_endpoint_label';

const DEFAULT_TARGET_HOST = 'localhost';

const OPTION_FLAGS: Readonly<Record<TunnelKindDto, string>> = {
	local: '-L',
	remote: '-R',
	dynamic: '-D',
};

function buildListenLabel(forward: Readonly<ExternalForwardDto>): string {
	const { listenAddress, listenPort } = forward;
	return listenAddress ? `${listenAddress}:${listenPort}` : String(listenPort);
}

/** Example: `3333 → localhost:3333`, or just `1080` for a dynamic forward. */
export function getExternalForwardRoute(
	forward: Readonly<ExternalForwardDto>,
	separator: string
): string {
	const listen = buildListenLabel(forward);
	if (forward.kind === 'dynamic') return listen;
	const targetHost = forward.targetHost ?? DEFAULT_TARGET_HOST;
	const targetPort = forward.targetPort ?? forward.listenPort;
	return `${listen} ${separator} ${targetHost}:${targetPort}`;
}

/** Example: `-L 3333 → localhost:3333`. */
export function getExternalForwardLabel(
	forward: Readonly<ExternalForwardDto>
): string {
	const route = getExternalForwardRoute(forward, VISUAL_SEPARATOR);
	return `${OPTION_FLAGS[forward.kind]} ${route}`;
}

/** The route in words, which screen readers read consistently: `3333 to localhost:3333`. */
export function getExternalForwardSpokenRoute(
	forward: Readonly<ExternalForwardDto>,
	i18n: I18n
): string {
	return getExternalForwardRoute(forward, i18n._(SPOKEN_SEPARATOR));
}

/** Example: `Local forward 3333 to localhost:3333`. */
export function getExternalForwardAccessibleLabel(
	forward: Readonly<ExternalForwardDto>,
	i18n: I18n
): string {
	const route = getExternalForwardSpokenRoute(forward, i18n);
	switch (forward.kind) {
		case 'local':
			return t(i18n)`Local forward ${route}`;
		case 'remote':
			return t(i18n)`Remote forward ${route}`;
		case 'dynamic':
			return t(i18n)`Dynamic proxy ${route}`;
	}
}
