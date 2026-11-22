import type {
	AvailableOpenerDto,
	ConfigImportEntryDto,
	ConfigImportPlanDto,
	DevslotDiscoveryDto,
	DevslotInstanceDto,
	DevslotPortDto,
	DevslotUpResultDto,
	DevslotStaleTunnelDto,
	ImportCandidateDto,
	ImportPreviewDto,
	X11AnchorDto,
	X11LauncherDto,
	ExternalProcessDto,
	MetricsDto,
	OpenerSpecDto,
	SettingsDto,
	SnapshotDto,
	TunnelDto,
} from '#/daemon/protocol';

const DEFAULT_LISTEN_PORT = 3333;
const MINUTE_IN_MILLISECONDS = 60_000;

export function buildMetrics(
	overrides: Readonly<Partial<MetricsDto>> = {}
): MetricsDto {
	return {
		activeConnections: 0,
		totalConnections: 0,
		bytesIn: 0,
		bytesOut: 0,
		...overrides,
	};
}

export function buildTunnel(
	overrides: Readonly<Partial<TunnelDto>> = {}
): TunnelDto {
	return {
		id: 'tunnel-1',
		host: 'nova',
		kind: 'local',
		listenPort: DEFAULT_LISTEN_PORT,
		tags: [],
		state: { kind: 'stopped' },
		health: { kind: 'unknown' },
		metrics: buildMetrics(),
		isAwake: false,
		isTemporary: false,
		defaultOpener: { id: 'browser', name: 'Browser' },
		...overrides,
	};
}

export function buildDevslotUpResult(
	overrides: Readonly<Partial<DevslotUpResultDto>> = {}
): DevslotUpResultDto {
	return {
		host: 'vega',
		instance: 'web-shop-098a',
		started: [{ remotePort: 20050, listenPort: 20050, tunnelId: 'tunnel-1' }],
		existing: [],
		skipped: [],
		failed: [],
		removed: [],
		needsReview: 0,
		appUrl: 'http://web-shop.localhost:20050',
		appPortShared: false,
		opened: 'http://web-shop.localhost:20050',
		...overrides,
	};
}

export function buildOpener(
	overrides: Readonly<Partial<AvailableOpenerDto>> = {}
): AvailableOpenerDto {
	return {
		id: 'remmina-vnc',
		name: 'Remmina (VNC)',
		command: 'remmina',
		args: ['-c', 'vnc://{host}:{port}'],
		ports: [5900],
		source: 'detected',
		...overrides,
	};
}

export function buildOpenerSpec(
	overrides: Readonly<Partial<OpenerSpecDto>> = {}
): OpenerSpecDto {
	return {
		id: 'dbeaver',
		name: 'DBeaver',
		command: 'dbeaver',
		args: ['-con', 'host={host}'],
		ports: [5432],
		...overrides,
	};
}

export function buildExternalProcess(
	overrides: Readonly<Partial<ExternalProcessDto>> = {}
): ExternalProcessDto {
	return {
		pid: 4242,
		process: 'ssh',
		destination: 'nova',
		forwards: [
			{
				kind: 'local',
				listenPort: DEFAULT_LISTEN_PORT,
				targetHost: 'localhost',
				targetPort: DEFAULT_LISTEN_PORT,
			},
		],
		...overrides,
	};
}

export function buildSettings(
	overrides: Readonly<Partial<SettingsDto>> = {}
): SettingsDto {
	return {
		defaultBindAddress: '127.0.0.1',
		defaultRemoteBindAddress: '127.0.0.1',
		keepaliveIntervalMs: MINUTE_IN_MILLISECONDS / 2,
		keepaliveMax: 3,
		sessionIdleGraceMs: MINUTE_IN_MILLISECONDS,
		reconnectInitialDelayMs: 1000,
		reconnectMaxDelayMs: MINUTE_IN_MILLISECONDS,
		restoreOnLaunch: true,
		startOnLaunchTags: [],
		launchAtLogin: false,
		useOsKeyring: false,
		openHost: 'localhost',
		x11ServerStart: 'ask',
		openDefault: 'suggested',
		standby: false,
		standbyIdleS: 300,
		standbyNotify: 'visual',
		standbySound: 'pluck',
		standbyVolume: 50,
		language: 'system',
		checkForUpdates: true,
		devslotPersist: false,
		devslotStale: 'ask',
		...overrides,
	};
}

export function buildSnapshot(
	overrides: Readonly<Partial<SnapshotDto>> = {}
): SnapshotDto {
	return {
		tunnels: [buildTunnel()],
		externals: [],
		x11Anchors: [],
		x11Launchers: [],
		openers: [],
		settings: buildSettings(),
		tray: { kind: 'active' },
		...overrides,
	};
}

export function buildX11Anchor(
	overrides: Readonly<Partial<X11AnchorDto>> = {}
): X11AnchorDto {
	return {
		host: 'nova',
		state: { kind: 'active', display: 'localhost:10.0' },
		...overrides,
	};
}

export function buildX11Launcher(
	overrides: Readonly<Partial<X11LauncherDto>> = {}
): X11LauncherDto {
	return {
		host: 'nova',
		name: 'VS Code',
		command: 'code --no-sandbox',
		...overrides,
	};
}

export function buildImportCandidate(
	overrides: Readonly<Partial<ImportCandidateDto>> = {}
): ImportCandidateDto {
	return {
		host: 'nova',
		kind: 'local',
		listenPort: DEFAULT_LISTEN_PORT,
		...overrides,
	};
}

export function buildImportPreview(
	overrides: Readonly<Partial<ImportPreviewDto>> = {}
): ImportPreviewDto {
	return { candidates: [], warnings: [], ...overrides };
}

export function buildConfigImportEntry(
	overrides: Readonly<Partial<ConfigImportEntryDto>> = {}
): ConfigImportEntryDto {
	return {
		index: 0,
		tunnel: { host: 'nova', kind: 'local', listenPort: 3333, tags: [] },
		fileHost: 'nova',
		effectiveListenAddress: '127.0.0.1',
		status: { kind: 'importable' },
		risks: [],
		warnings: [],
		selectedByDefault: true,
		...overrides,
	};
}

export function buildConfigImportPlan(
	overrides: Readonly<Partial<ConfigImportPlanDto>> = {}
): ConfigImportPlanDto {
	return { source: 'export', entries: [], unknownHosts: [], ...overrides };
}

export function buildDevslotPort(
	overrides: Readonly<Partial<DevslotPortDto>> = {}
): DevslotPortDto {
	return {
		var: 'PORT',
		port: 20_050,
		kind: 'web',
		forced: false,
		status: 'new',
		...overrides,
	};
}

export function buildDevslotInstance(
	overrides: Readonly<Partial<DevslotInstanceDto>> = {}
): DevslotInstanceDto {
	return {
		name: 'web-shop-feat-auth-3f2a',
		project: 'web-shop',
		projectTag: 'ds-web-shop',
		path: '/home/me/dev/web-shop.feat-auth',
		ports: [buildDevslotPort()],
		...overrides,
	};
}

export function buildDevslotStale(
	overrides: Readonly<Partial<DevslotStaleTunnelDto>> = {}
): DevslotStaleTunnelDto {
	return {
		tunnel: {
			id: 'stale-1',
			host: 'vega',
			kind: 'local',
			listenPort: 20_060,
			tags: ['devslot'],
		},
		isTemporary: false,
		reason: 'portGone',
		action: 'ask',
		...overrides,
	};
}

export function buildDevslotDiscovery(
	overrides: Readonly<Partial<DevslotDiscoveryDto>> = {}
): DevslotDiscoveryDto {
	return {
		host: 'vega',
		persistDefault: false,
		instances: [buildDevslotInstance()],
		stale: [],
		warnings: [],
		...overrides,
	};
}
