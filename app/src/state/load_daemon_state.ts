import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { useDaemonStore } from '#/state/daemon_store';

async function loadHosts(gateway: DaemonGateway): Promise<readonly string[]> {
	try {
		return await gateway.listHosts();
	} catch {
		return [];
	}
}

/** Reads the snapshot and the hosts into the store; hosts are optional. */
export async function loadDaemonState(gateway: DaemonGateway): Promise<void> {
	const [snapshot, hosts] = await Promise.all([
		gateway.snapshot(),
		loadHosts(gateway),
	]);
	const { applySnapshot, setHosts } = useDaemonStore.getState();
	applySnapshot(snapshot);
	setHosts(hosts);
}
