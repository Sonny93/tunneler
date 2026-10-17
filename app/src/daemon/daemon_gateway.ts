import type {
	EventMessage,
	HostKeyDecisionDto,
	ImportPreviewDto,
	LogEntryDto,
	ResolvedHostDto,
	SettingsDto,
	SnapshotDto,
	TunnelDefinitionDto,
	TunnelDto,
	TunnelSpecDto,
} from '#/daemon/protocol';

export type Unsubscribe = () => void;

/** A secret, the answers of a keyboard-interactive round, or `null` when the user cancelled. */
export type CredentialAnswer = string | readonly string[] | null;

export type DaemonGateway = {
	/** Ensures the daemon runs (spawns it if needed) and connects; idempotent. */
	connect(): Promise<void>;
	snapshot(): Promise<SnapshotDto>;
	listHosts(): Promise<readonly string[]>;
	createTunnel(spec: TunnelSpecDto): Promise<TunnelDto>;
	updateTunnel(definition: TunnelDefinitionDto): Promise<TunnelDto>;
	deleteTunnel(id: string): Promise<void>;
	startTunnel(id: string): Promise<void>;
	stopTunnel(id: string): Promise<void>;
	restartTunnel(id: string): Promise<void>;
	startTag(tag: string): Promise<void>;
	stopTag(tag: string): Promise<void>;
	startX11(host: string): Promise<void>;
	stopX11(host: string): Promise<void>;
	launchX11(host: string, name: string): Promise<void>;
	tunnelLogs(id: string): Promise<readonly LogEntryDto[]>;
	resolveHost(host: string): Promise<ResolvedHostDto>;
	importSshConfig(): Promise<ImportPreviewDto>;
	importScript(text: string): Promise<ImportPreviewDto>;
	/** The native side only opens loopback names and IP literals. */
	openInBrowser(host: string, port: number): Promise<void>;
	adoptExternal(id: string): Promise<readonly string[]>;
	killExternal(pid: number): Promise<void>;
	ignoreExternal(pid: number): Promise<void>;
	updateSettings(settings: SettingsDto): Promise<void>;
	respondCredential(promptId: string, answer: CredentialAnswer): Promise<void>;
	respondHostKey(promptId: string, decision: HostKeyDecisionDto): Promise<void>;
	/** On success the native side exits the app. */
	quit(force: boolean): Promise<void>;
	onEvent(listener: (event: EventMessage) => void): Promise<Unsubscribe>;
	onDisconnected(listener: (message: string) => void): Promise<Unsubscribe>;
};
