import { vi, type Mock } from 'vitest';

import type { DaemonGateway } from '#/daemon/daemon_gateway';
import type { EventMessage, SnapshotDto } from '#/daemon/protocol';
import { buildSnapshot, buildTunnel } from '#/testing/factories';

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
		startTemporaryTunnel: vi.fn((spec) =>
			Promise.resolve(
				buildTunnel({
					...spec,
					tags: [],
					id: 'temporary-tunnel',
					isTemporary: true,
				})
			)
		),
		persistTunnel: vi.fn(() => Promise.resolve()),
		checkTarget: vi.fn(() => Promise.resolve({ kind: 'reachable' as const })),
		previewSound: vi.fn(() => Promise.resolve()),
		deleteTunnel: vi.fn(() => Promise.resolve()),
		startTunnel: vi.fn(() => Promise.resolve()),
		stopTunnel: vi.fn(() => Promise.resolve()),
		restartTunnel: vi.fn(() => Promise.resolve()),
		startTag: vi.fn(() => Promise.resolve()),
		stopTag: vi.fn(() => Promise.resolve()),
		standbyTunnel: vi.fn(() => Promise.resolve()),
		standbyTag: vi.fn(() => Promise.resolve()),
		standbyAll: vi.fn(() => Promise.resolve()),
		renameTag: vi.fn(() => Promise.resolve(0)),
		deleteTag: vi.fn(() => Promise.resolve({ removedCount: 0 })),
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
		updateOpeners: vi.fn(() => Promise.resolve([])),
		openTunnel: vi.fn(() =>
			Promise.resolve({ target: 'http://localhost:3333/' })
		),
		adoptExternal: vi.fn(() => Promise.resolve([])),
		killExternal: vi.fn(() => Promise.resolve()),
		ignoreExternal: vi.fn(() => Promise.resolve()),
		updateSettings: vi.fn(() => Promise.resolve()),
		respondCredential: vi.fn(() => Promise.resolve()),
		forgetSecrets: vi.fn(() => Promise.resolve({ forgottenCount: 0 })),
		revealConfig: vi.fn(() => Promise.resolve()),
		checkForUpdate: vi.fn(() => Promise.resolve(null)),
		installUpdate: vi.fn(() => Promise.resolve()),
		openReleasePage: vi.fn(() => Promise.resolve()),
		openDocumentation: vi.fn(() => Promise.resolve()),
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
