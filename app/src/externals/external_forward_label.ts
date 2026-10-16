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

const SPOKEN_KINDS: Readonly<Record<TunnelKindDto, string>> = {
	local: 'Redirection locale',
	remote: 'Redirection distante',
	dynamic: 'Proxy dynamique',
};

function buildListenLabel(forward: Readonly<ExternalForwardDto>): string {
	const { listenAddress, listenPort } = forward;
	return listenAddress ? `${listenAddress}:${listenPort}` : String(listenPort);
}

/** Example: `3333 vers localhost:3333`, or just `1080` for a dynamic forward. */
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

/** Example: `Redirection locale 3333 vers localhost:3333`. */
export function getExternalForwardAccessibleLabel(
	forward: Readonly<ExternalForwardDto>
): string {
	const route = getExternalForwardRoute(forward, SPOKEN_SEPARATOR);
	return `${SPOKEN_KINDS[forward.kind]} ${route}`;
}
