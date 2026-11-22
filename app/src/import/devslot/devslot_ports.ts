import type { DevslotInstanceDto, DevslotPortDto } from '#/daemon/protocol';

export const PRIVILEGED_PORT_LIMIT = 1024;
const WEB_PORT_KIND = 'web';

export type DevslotPortEntry = {
	/** Position of the port in the discovery: `<instance index>:<port index>`. */
	key: string;
	instance: DevslotInstanceDto;
	port: DevslotPortDto;
};

export function flattenDevslotPorts(
	instances: readonly Readonly<DevslotInstanceDto>[]
): DevslotPortEntry[] {
	return instances.flatMap((instance, instanceIndex) =>
		instance.ports.map((port, portIndex) => ({
			key: `${instanceIndex}:${portIndex}`,
			instance,
			port,
		}))
	);
}

export function isWebPort(port: Readonly<DevslotPortDto>): boolean {
	return port.kind === WEB_PORT_KIND;
}

/** A port a tunnel already forwards to is shown but cannot be chosen. */
export function isPortTunnelable(entry: Readonly<DevslotPortEntry>): boolean {
	return entry.port.status !== 'alreadyTunneled';
}

/** Web ports above the privileged range are checked at first; services are left to the user. */
export function isPortPreselected(entry: Readonly<DevslotPortEntry>): boolean {
	return (
		isPortTunnelable(entry) &&
		isWebPort(entry.port) &&
		entry.port.port >= PRIVILEGED_PORT_LIMIT
	);
}
