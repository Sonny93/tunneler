import type {
	EventMessage,
	HostKeyDecisionDto,
	ImportPreviewDto,
	DeleteTagResultDto,
	AvailableOpenerDto,
	OpenerSpecDto,
	ForgetSecretsResultDto,
	LogEntryDto,
	OpenTunnelResultDto,
	RememberDto,
	ResolvedHostDto,
	SettingsDto,
	SnapshotDto,
	TargetHealthDto,
	TunnelDefinitionDto,
	TunnelDto,
	TemporaryTunnelSpecDto,
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
	/** Starts a tunnel that lives only in the daemon: never saved, removed when stopped. */
	startTemporaryTunnel(spec: TemporaryTunnelSpecDto): Promise<TunnelDto>;
	/** Saves a temporary tunnel; it keeps running. */
	persistTunnel(id: string): Promise<void>;
	/** Tries the target of a running local or remote tunnel once, and resolves to its health. */
	checkTarget(id: string): Promise<TargetHealthDto>;
	deleteTunnel(id: string): Promise<void>;
	startTunnel(id: string): Promise<void>;
	stopTunnel(id: string): Promise<void>;
	restartTunnel(id: string): Promise<void>;
	startTag(tag: string): Promise<void>;
	stopTag(tag: string): Promise<void>;
	/** Renames a tag in every tunnel and in the launch tags; resolves to the number of tunnels that carried it. */
	renameTag(from: string, to: string): Promise<number>;
	/** Removes a tag from every tunnel and from the launch tags, keeping the tunnels. */
	deleteTag(tag: string): Promise<DeleteTagResultDto>;
	startX11(host: string): Promise<void>;
	stopX11(host: string): Promise<void>;
	launchX11(host: string, name: string): Promise<void>;
	tunnelLogs(id: string): Promise<readonly LogEntryDto[]>;
	resolveHost(host: string): Promise<ResolvedHostDto>;
	importSshConfig(): Promise<ImportPreviewDto>;
	importScript(text: string): Promise<ImportPreviewDto>;
	/** The daemon builds the URL, starts a stopped tunnel and waits until it is ready, then opens it with `opener` (`browser` or an id) or the default action. */
	openTunnel(id: string, opener?: string): Promise<OpenTunnelResultDto>;
	/** Replaces the openers added by the user; resolves to every available opener. */
	updateOpeners(
		openers: readonly OpenerSpecDto[]
	): Promise<readonly AvailableOpenerDto[]>;
	adoptExternal(id: string): Promise<readonly string[]>;
	killExternal(pid: number): Promise<void>;
	ignoreExternal(pid: number): Promise<void>;
	updateSettings(settings: SettingsDto): Promise<void>;
	respondCredential(
		promptId: string,
		answer: CredentialAnswer,
		remember?: RememberDto
	): Promise<void>;
	forgetSecrets(): Promise<ForgetSecretsResultDto>;
	/** Shows the configuration file in the file manager; the path comes from the daemon. */
	revealConfig(): Promise<void>;
	respondHostKey(promptId: string, decision: HostKeyDecisionDto): Promise<void>;
	/** On success the native side exits the app. */
	quit(force: boolean): Promise<void>;
	onEvent(listener: (event: EventMessage) => void): Promise<Unsubscribe>;
	onDisconnected(listener: (message: string) => void): Promise<Unsubscribe>;
};
