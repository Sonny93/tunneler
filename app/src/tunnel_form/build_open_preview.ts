import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type {
	AvailableOpenerDto,
	OpenHostDto,
	SettingsDto,
} from '#/daemon/protocol';
import { BROWSER_OPENER_ID } from '#/openers/open_choices';
import { parsePort } from '#/tunnel_form/parse_port';
import { suggestOpener } from '#/tunnel_form/suggest_opener';
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

type OpenAddress = { host: string; port: number; url: string };

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

function buildAddress(
	values: Readonly<TunnelFormValues>,
	settings: Readonly<SettingsDto> | null
): OpenAddress | null | 'hostname' {
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
	if (host === null) return 'hostname';
	const path = values.openPath.trim() || DEFAULT_PATH;
	return { host, port, url: `${values.openScheme}://${host}:${port}${path}` };
}

function quoteForDisplay(text: string): string {
	return /\s/u.test(text) ? `"${text}"` : text;
}

function describeCommand(
	opener: Readonly<AvailableOpenerDto>,
	address: Readonly<OpenAddress>
): string {
	const args = opener.args.map((argument) =>
		argument
			.replaceAll('{host}', address.host)
			.replaceAll('{port}', String(address.port))
			.replaceAll('{url}', address.url)
	);
	return [opener.command, ...args].map(quoteForDisplay).join(' ');
}

function findChosenOpener(
	values: Readonly<TunnelFormValues>,
	openers: readonly Readonly<AvailableOpenerDto>[]
): Readonly<AvailableOpenerDto> | null {
	if (values.openWith === BROWSER_OPENER_ID) return null;
	if (values.openWith === '') return suggestOpener(values, openers);
	return openers.find(({ id }) => id === values.openWith) ?? null;
}

/**
 * The sentence previewing what "Open" does: the URL for the browser, the
 * command line for an application. `null` while the listen port is invalid.
 * Display only: the daemon builds the real URL and command.
 */
export function buildOpenPreview(
	values: Readonly<TunnelFormValues>,
	settings: Readonly<SettingsDto> | null,
	openers: readonly Readonly<AvailableOpenerDto>[],
	i18n: I18n
): string | null {
	const address = buildAddress(values, settings);
	if (address === null) return null;
	if (address === 'hostname') {
		return t(
			i18n
		)`The listen address is a host name: the tunnel can't be opened.`;
	}
	const opener = findChosenOpener(values, openers);
	if (opener === null) {
		const { url } = address;
		return t(i18n)`Opens ${url}`;
	}
	const command = describeCommand(opener, address);
	return t(i18n)`Runs ${command}`;
}
