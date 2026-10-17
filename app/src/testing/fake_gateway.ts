import { vi, type Mock } from 'vitest';

import type { DaemonGateway } from '#/daemon/daemon_gateway';
import { buildSnapshot, buildTunnel } from '#/testing/factories';
import type { EventMessage, SnapshotDto } from '#/daemon/protocol';

export type FakeGateway = {
	[Method in keyof DaemonGateway]: Mock<DaemonGateway[Method]>;
} & {
	emit(event: EventMessage): void;
	disconnect(message: string): void;
};

type FakeGatewayOptions = {
	snapshot?: SnapshotDto;
	hosts?: readonly string[];
};

export function createFakeGateway(
	options: Readonly<FakeGatewayOptions> = {}
): FakeGateway {
	const eventListeners = new Set<(event: EventMessage) => void>();
	const disconnectListeners = new Set<(message: string) => void>();
	const snapshot = options.snapshot ?? buildSnapshot();
	const hosts = options.hosts ?? [];

	return {
		connect: vi.fn(() => Promise.resolve()),
		snapshot: vi.fn(() => Promise.resolve(snapshot)),
		listHosts: vi.fn(() => Promise.resolve(hosts)),
		createTunnel: vi.fn((spec) =>
			Promise.resolve(buildTunnel({ ...spec, id: 'created-tunnel' }))
		),
		updateTunnel: vi.fn((definition) =>
			Promise.resolve(buildTunnel(definition))
		),
		deleteTunnel: vi.fn(() => Promise.resolve()),
		startTunnel: vi.fn(() => Promise.resolve()),
		stopTunnel: vi.fn(() => Promise.resolve()),
		restartTunnel: vi.fn(() => Promise.resolve()),
		startTag: vi.fn(() => Promise.resolve()),
		stopTag: vi.fn(() => Promise.resolve()),
		startX11: vi.fn(() => Promise.resolve()),
		stopX11: vi.fn(() => Promise.resolve()),
		launchX11: vi.fn(() => Promise.resolve()),
		tunnelLogs: vi.fn(() => Promise.resolve([])),
		resolveHost: vi.fn((host) =>
			Promise.resolve({ host, options: [], warnings: [] })
		),
		importSshConfig: vi.fn(() =>
			Promise.resolve({ candidates: [], warnings: [] })
		),
		importScript: vi.fn(() =>
			Promise.resolve({ candidates: [], warnings: [] })
		),
		openInBrowser: vi.fn(() => Promise.resolve()),
		adoptExternal: vi.fn(() => Promise.resolve([])),
		killExternal: vi.fn(() => Promise.resolve()),
		ignoreExternal: vi.fn(() => Promise.resolve()),
		updateSettings: vi.fn(() => Promise.resolve()),
		respondCredential: vi.fn(() => Promise.resolve()),
		respondHostKey: vi.fn(() => Promise.resolve()),
		quit: vi.fn(() => Promise.resolve()),
		onEvent: vi.fn((listener) => {
			eventListeners.add(listener);
			return Promise.resolve(() => void eventListeners.delete(listener));
		}),
		onDisconnected: vi.fn((listener) => {
			disconnectListeners.add(listener);
			return Promise.resolve(() => void disconnectListeners.delete(listener));
		}),
		emit: (event) => {
			for (const listener of eventListeners) listener(event);
		},
		disconnect: (message) => {
			for (const listener of disconnectListeners) listener(message);
		},
	};
}
