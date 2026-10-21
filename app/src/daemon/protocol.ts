/**
 * Hand-written mirror of the wire DTOs in `crates/ipc/src/protocol/dto.rs` and
 * `event.rs`. The Rust types are the source of truth: the Rust bridge
 * deserializes every message with serde before forwarding it to the webview,
 * so no runtime schema validation happens here. Timestamps are epoch
 * milliseconds; optional fields are omitted when absent.
 */

export type EpochMillis = number;

export type TunnelKindDto = 'local' | 'remote' | 'dynamic';

export type OpenSchemeDto = 'http' | 'https';

export type OpenHostDto = 'localhost' | 'ip';

/** How a tunnel is opened; every field is optional and `open` is absent when none is set. */
export type OpenOptionsDto = {
	scheme?: OpenSchemeDto;
	path?: string;
	host?: OpenHostDto;
};

export type TunnelSpecDto = {
	host: string;
	kind: TunnelKindDto;
	listenPort: number;
	listenAddress?: string;
	targetHost?: string;
	targetPort?: number;
	tags: string[];
	open?: OpenOptionsDto;
};

/** What `tunnel.open` opened: the URL for the browser. */
export type OpenTunnelResultDto = { target: string };

/** A temporary tunnel is never selected by tag, so it has none. */
export type TemporaryTunnelSpecDto = Omit<TunnelSpecDto, 'tags' | 'open'>;

export type TunnelDefinitionDto = TunnelSpecDto & { id: string };

export type FailureReasonDto =
	| { kind: 'portInUse'; address: string }
	| { kind: 'listenFailed'; address: string; message: string }
	| { kind: 'connect'; message: string }
	| { kind: 'remoteForwardRefused'; address: string }
	| { kind: 'agentUnresponsive'; timeoutMs: number };

export type ProcessOwnerDto = { pid: number; process: string };

export type TunnelStateDto =
	| { kind: 'stopped' }
	| { kind: 'starting' }
	| { kind: 'active' }
	| { kind: 'reconnecting'; attempt: number; nextRetryAt: EpochMillis }
	| { kind: 'failed'; reason: FailureReasonDto }
	| { kind: 'external'; owner: ProcessOwnerDto }
	| { kind: 'conflict'; owner?: ProcessOwnerDto };

export type TargetHealthDto =
	| { kind: 'unknown' }
	| { kind: 'reachable' }
	| { kind: 'unreachable'; since: EpochMillis; error: string };

export type MetricsDto = {
	activeConnections: number;
	totalConnections: number;
	bytesIn: number;
	bytesOut: number;
	lastActivity?: EpochMillis;
};

export type TunnelDto = TunnelDefinitionDto & {
	state: TunnelStateDto;
	health: TargetHealthDto;
	metrics: MetricsDto;
	/** Lives only in the daemon: never saved, and stopping it removes it. */
	isTemporary: boolean;
};

export type LogLevelDto = 'info' | 'warn' | 'error';

export type LogEntryDto = {
	at: EpochMillis;
	level: LogLevelDto;
	message: string;
};

export type ResolvedHostOptionDto = { key: string; value: string };

export type ResolvedHostDto = {
	host: string;
	options: ResolvedHostOptionDto[];
	/** English text. */
	warnings: string[];
};

export type ImportCandidateDto = {
	host?: string;
	kind: TunnelKindDto;
	listenPort: number;
	listenAddress?: string;
	targetHost?: string;
	targetPort?: number;
	comment?: string;
	line?: number;
};

export type ImportWarningDto =
	| { kind: 'malformedForward'; line: number; text: string }
	| { kind: 'unsupportedForward'; line?: number; host?: string; text: string }
	| { kind: 'unknownDestination'; destination: string }
	| { kind: 'hostResolutionFailed'; host: string; message: string };

export type ImportPreviewDto = {
	candidates: ImportCandidateDto[];
	warnings: ImportWarningDto[];
};

export type SettingsDto = {
	defaultBindAddress: string;
	defaultRemoteBindAddress: string;
	keepaliveIntervalMs: number;
	keepaliveMax: number;
	sessionIdleGraceMs: number;
	reconnectInitialDelayMs: number;
	reconnectMaxDelayMs: number;
	restoreOnLaunch: boolean;
	startOnLaunchTags: string[];
	launchAtLogin: boolean;
	useOsKeyring: boolean;
	x11Display?: string;
	openHost: OpenHostDto;
};

export type TrayStatusDto =
	| { kind: 'active' }
	| { kind: 'missingHost' }
	| { kind: 'failed'; message: string }
	| { kind: 'disabled' };

export type ExternalForwardDto = {
	kind: TunnelKindDto;
	listenAddress?: string;
	listenPort: number;
	targetHost?: string;
	targetPort?: number;
};

export type ExternalProcessDto = {
	pid: number;
	process: string;
	destination?: string;
	forwards: ExternalForwardDto[];
};

export type X11StateDto =
	| { kind: 'stopped' }
	| { kind: 'starting' }
	| { kind: 'active'; display: string }
	| { kind: 'reconnecting'; attempt: number; nextRetryAt: EpochMillis }
	| { kind: 'failed'; message: string };

/** Only anchors that are not stopped are listed. */
export type X11AnchorDto = { host: string; state: X11StateDto };

export type X11LauncherDto = { host: string; name: string; command: string };

export type SnapshotDto = {
	tunnels: TunnelDto[];
	externals: ExternalProcessDto[];
	x11Anchors: X11AnchorDto[];
	x11Launchers: X11LauncherDto[];
	settings: SettingsDto;
	tray: TrayStatusDto;
};

export type PromptDto = { text: string; echo: boolean };

export type CredentialRequestDto =
	| { kind: 'passphrase'; alias: string; keyPath: string }
	| { kind: 'password'; alias: string; user: string; host: string }
	| {
			kind: 'keyboardInteractive';
			alias: string;
			name: string;
			instructions: string;
			prompts: PromptDto[];
	  };

/** How long the daemon keeps a passphrase the user just typed. */
export type RememberDto = 'session' | 'keyring';

export type ForgetSecretsResultDto = { forgottenCount: number };

export type DeleteTagResultDto = { removedCount: number };

export type HostKeyDecisionDto = 'acceptAndSave' | 'acceptOnce' | 'reject';

export type HostKeyAlertReasonDto =
	| { kind: 'changed'; knownFingerprint: string; path: string; line: number }
	| { kind: 'revoked'; path: string; line: number }
	| { kind: 'unknownRejectedByPolicy' };

export type HostKeyRequestEventData =
	| {
			kind: 'prompt';
			promptId: string;
			alias: string;
			lookupName: string;
			algorithm: string;
			fingerprintSha256: string;
	  }
	| {
			kind: 'alert';
			alias: string;
			reason: HostKeyAlertReasonDto;
			presentedFingerprint: string;
	  };

export type EventMessage =
	| {
			event: 'tunnel.changed';
			data: { id: string; state: TunnelStateDto; health: TargetHealthDto };
	  }
	| { event: 'tunnel.metrics'; data: { id: string; metrics: MetricsDto } }
	| { event: 'tunnel.log'; data: { id: string; entry: LogEntryDto } }
	| { event: 'config.reloaded'; data: Record<string, never> }
	| { event: 'config.invalid'; data: { messages: string[] } }
	| {
			event: 'credential.request';
			data: { promptId: string } & CredentialRequestDto;
	  }
	| { event: 'hostkey.request'; data: HostKeyRequestEventData }
	| { event: 'quit.requested'; data: Record<string, never> }
	| { event: 'tray.changed'; data: { status: TrayStatusDto } }
	| { event: 'show.requested'; data: Record<string, never> }
	| { event: 'externals.changed'; data: { processes: ExternalProcessDto[] } }
	| { event: 'prompt.resolved'; data: { promptId: string } }
	| { event: 'x11.changed'; data: { host: string; state: X11StateDto } };
