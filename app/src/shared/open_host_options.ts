import type { OpenHostDto } from '#/daemon/protocol';

export const OPEN_HOST_OPTIONS = [
	{
		value: 'localhost',
		label: 'localhost',
		description: 'Le nom localhost',
	},
	{
		value: 'ip',
		label: 'Adresse IP',
		description: '127.0.0.1, ou [::1] quand le tunnel écoute en IPv6',
	},
] as const satisfies readonly {
	value: OpenHostDto;
	label: string;
	description: string;
}[];

export function describeOpenHost(host: OpenHostDto): string {
	return host === 'ip' ? 'Adresse IP' : 'localhost';
}
