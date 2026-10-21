import { parsePort } from '#/tunnel_form/parse_port';
import type { OpenHostDto, SettingsDto } from '#/daemon/protocol';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';

const LOCALHOST = 'localhost';
const IPV4_LOOPBACK = '127.0.0.1';
const IPV6_LOOPBACK = '[::1]';
const DEFAULT_PATH = '/';
const LOOPBACK_OR_WILDCARD_ADDRESSES = [
	'',
	'*',
	'0.0.0.0',
	'::',
	'::1',
	'127.0.0.1',
	'localhost',
];
const IPV4_PATTERN = /^(\d{1,3})(\.\d{1,3}){3}$/u;
const IPV6_PATTERN = /^[0-9a-f:.]+$/iu;
const MAX_IPV4_OCTET = 255;
const HOSTNAME_MESSAGE =
	"L'adresse d'écoute est un nom d'hôte : le tunnel ne peut pas être ouvert.";

function isIpv4(address: string): boolean {
	return (
		IPV4_PATTERN.test(address) &&
		address.split('.').every((octet) => Number(octet) <= MAX_IPV4_OCTET)
	);
}

function isIpv6(address: string): boolean {
	return address.includes(':') && IPV6_PATTERN.test(address);
}

function pickLoopbackHost(address: string, preference: OpenHostDto): string {
	if (preference === 'localhost') return LOCALHOST;
	return address.includes(':') ? IPV6_LOOPBACK : IPV4_LOOPBACK;
}

function pickHost(address: string, preference: OpenHostDto): string | null {
	if (LOOPBACK_OR_WILDCARD_ADDRESSES.includes(address.toLowerCase())) {
		return pickLoopbackHost(address, preference);
	}
	if (isIpv4(address)) return address;
	if (isIpv6(address)) return `[${address}]`;
	return null;
}

/**
 * The sentence previewing where the tunnel opens, or `null` while the listen
 * port is invalid. Display only: the daemon builds the real URL.
 */
export function buildOpenPreview(
	values: Readonly<TunnelFormValues>,
	settings: Readonly<SettingsDto> | null
): string | null {
	const port = parsePort(values.listenPort);
	if (port === null) return null;
	const address = (
		(values.listenAddress.trim() || settings?.defaultBindAddress) ??
		''
	).replaceAll(/[[\]]/gu, '');
	const preference =
		values.openHost === ''
			? (settings?.openHost ?? 'localhost')
			: values.openHost;
	const host = pickHost(address, preference);
	if (host === null) return HOSTNAME_MESSAGE;
	const path = values.openPath.trim() || DEFAULT_PATH;
	return `Ouvre ${values.openScheme}://${host}:${port}${path}`;
}
