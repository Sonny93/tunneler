import type { ImportConflictDto, ImportRouteDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';

/** A stand-in for `import.classify`: compares each route with the tunnels of the daemon store only. */
export function classifyAgainstStore(
	routes: readonly ImportRouteDto[]
): Promise<(ImportConflictDto | null)[]> {
	const { tunnels } = useDaemonStore.getState();
	return Promise.resolve(
		routes.map((route) => {
			const holder = tunnels.find(
				(tunnel) => tunnel.listenPort === route.listenPort
			);
			if (holder === undefined) return null;
			const isSameRoute =
				holder.host === route.host &&
				(holder.targetPort ?? holder.listenPort) ===
					(route.targetPort ?? route.listenPort);
			return isSameRoute
				? { kind: 'alreadyConfigured' }
				: { kind: 'listenConflict', holderId: holder.id };
		})
	);
}
