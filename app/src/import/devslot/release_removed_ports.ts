import type { DevslotInstanceDto } from '#/daemon/protocol';

/** A port whose tunnel was removed since the discovery is new again: it can be chosen like any other. */
export function releaseRemovedPorts(
	instances: readonly Readonly<DevslotInstanceDto>[],
	removedIds: ReadonlySet<string>
): DevslotInstanceDto[] {
	return instances.map((instance) => ({
		...instance,
		ports: instance.ports.map((port) => {
			const isRemoved =
				port.status === 'alreadyTunneled' &&
				port.tunnelId !== undefined &&
				removedIds.has(port.tunnelId);
			if (!isRemoved) return port;
			const { tunnelId: _tunnelId, ...released } = port;
			return { ...released, status: 'new' };
		}),
	}));
}
