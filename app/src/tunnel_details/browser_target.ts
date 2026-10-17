import type { TunnelDefinitionDto } from '#/daemon/protocol';

const LOCAL_BROWSER_HOST = 'localhost';
const WILDCARD_ADDRESSES: readonly string[] = ['', '0.0.0.0', '::', '*'];

export type BrowserTarget = { host: string; port: number };

type BrowserTargetSource = Pick<
	TunnelDefinitionDto,
	'kind' | 'listenPort' | 'listenAddress'
>;

/**
 * Where a browser reaches a local forward, or `null` for other kinds. A wildcard
 * bind address is reached through `localhost`.
 */
export function getBrowserTarget(
	tunnel: Readonly<BrowserTargetSource>,
	defaultBindAddress: string | undefined
): BrowserTarget | null {
	if (tunnel.kind !== 'local') return null;
	const address = tunnel.listenAddress ?? defaultBindAddress ?? '';
	const host = WILDCARD_ADDRESSES.includes(address)
		? LOCAL_BROWSER_HOST
		: address;
	return { host, port: tunnel.listenPort };
}
