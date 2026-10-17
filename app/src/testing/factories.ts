import type {
	ImportCandidateDto,
	ImportPreviewDto,
	X11AnchorDto,
	X11LauncherDto,
	ExternalProcessDto,
	MetricsDto,
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
		name: 'Adonis',
		host: 'nova',
		kind: 'local',
		listenPort: DEFAULT_LISTEN_PORT,
		tags: [],
		state: { kind: 'stopped' },
		health: { kind: 'unknown' },
		metrics: buildMetrics(),
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
		healthProbe: true,
		restoreOnLaunch: true,
		startOnLaunchTags: [],
		launchAtLogin: false,
		useOsKeyring: false,
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
