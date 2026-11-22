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

export type OpenDefaultDto = 'suggested' | 'browser';

/** What to do when no X server answers on the local display. */
export type X11ServerStartDto = 'ask' | 'always' | 'never';

/** How a tunnel is opened; every field is optional and `open` is absent when none is set. */
export type OpenOptionsDto = {
	scheme?: OpenSchemeDto;
	path?: string;
	host?: OpenHostDto;
	/** `browser` or the id of an opener; absent means the suggestion for the target port. */
	with?: string;
	/** A `<label>.localhost` name the URL uses instead of the loopback address; set by devslot discovery. */
	hostName?: string;
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
	/** Absent inherits the general `standby` setting; ignored for a remote tunnel. */
	standby?: boolean;
};

/** What `tunnel.open` opened: the URL for the browser. */
export type OpenTunnelResultDto = { target: string };

/** Where a temporary tunnel comes from when devslot discovery started it; kept in memory only. */
export type DevslotOriginDto = { projectTag?: string };

/** A temporary tunnel is never selected by tag, so it has none. */
export type TemporaryTunnelSpecDto = Omit<TunnelSpecDto, 'tags' | 'standby'> & {
	devslot?: DevslotOriginDto;
};

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
	| { kind: 'standby'; lastFailure?: FailureReasonDto }
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

export type OpenerSpecDto = {
	id: string;
	name: string;
	command: string;
	args: string[];
	ports: number[];
};

export type OpenerSourceDto = 'detected' | 'user';

export type AvailableOpenerDto = OpenerSpecDto & { source: OpenerSourceDto };

/** What a tunnel opens with when nothing is asked: `browser` or an opener id. */
export type DefaultOpenerDto = { id: string; name: string };

export type TunnelDto = TunnelDefinitionDto & {
	state: TunnelStateDto;
	health: TargetHealthDto;
	metrics: MetricsDto;
	/** Woken from standby and serving: it can be sent back to standby. */
	isAwake: boolean;
	/** Lives only in the daemon: never saved, and stopping it removes it. */
	isTemporary: boolean;
	defaultOpener: DefaultOpenerDto;
	/** Present only for a temporary tunnel started by devslot discovery. */
	devslot?: DevslotOriginDto;
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

export type StandbyNotifyDto = 'none' | 'visual' | 'sound' | 'both';

export type StandbySoundDto = 'system' | 'chime' | 'marimba' | 'pluck';

export type LanguageSettingDto = 'system' | 'en' | 'fr';

export type SoundEventDto = 'woke' | 'wakeFailed';

/** What happens to a saved devslot tunnel whose port is no longer listed. */
export type DevslotStalePolicyDto = 'ask' | 'delete';

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
	x11ServerStart: X11ServerStartDto;
	openHost: OpenHostDto;
	openDefault: OpenDefaultDto;
	standby: boolean;
	standbyIdleS: number;
	standbyNotify: StandbyNotifyDto;
	standbySound: StandbySoundDto;
	/** Percent of the synthesized sounds; the system sound follows the system volume. */
	standbyVolume: number;
	language: LanguageSettingDto;
	checkForUpdates: boolean;
	devslotPersist: boolean;
	devslotStale: DevslotStalePolicyDto;
};

export type DevslotPortStatusDto = 'new' | 'ambiguous' | 'alreadyTunneled';

export type DevslotPortDto = {
	var: string;
	port: number;
	/** `web` or `service`; any other value is treated like `service`. */
	kind: string;
	service?: string;
	target?: number;
	url?: string;
	hostName?: string;
	forced: boolean;
	status: DevslotPortStatusDto;
	tunnelId?: string;
	/** Present only when true: another program already listens on the same local port. */
	localPortBusy?: boolean;
	/** A free local port the daemon found; may be absent. */
	suggestedLocalPort?: number;
};

export type DevslotInstanceDto = {
	name: string;
	project?: string;
	projectTag?: string;
	path: string;
	worktree?: boolean;
	slot?: number;
	appUrl?: string;
	servicesRunning?: boolean;
	ports: DevslotPortDto[];
};

export type DevslotStaleReasonDto = 'portGone' | 'reassigned';

export type DevslotStaleActionDto = 'remove' | 'ask';

export type DevslotStaleTunnelDto = {
	tunnel: TunnelDefinitionDto;
	isTemporary: boolean;
	reason: DevslotStaleReasonDto;
	action: DevslotStaleActionDto;
};

export type DevslotWarningDto = {
	kind: 'invalidPort' | 'projectWithoutTag';
	instance: string;
	var?: string;
};

export type DevslotDiscoveryDto = {
	host: string;
	/** The host override, else the `devslotPersist` setting. */
	persistDefault: boolean;
	instances: DevslotInstanceDto[];
	stale: DevslotStaleTunnelDto[];
	warnings: DevslotWarningDto[];
};

export type DevslotResyncTunnelDto = {
	id: string;
	definition: TunnelDefinitionDto;
	isTemporary: boolean;
};

export type DevslotResyncResultDto = {
	removed: string[];
	skipped: string[];
};

/** What a window relays from a `tunneler://devslot/<host>/<instance>` link; the text of an invalid link is never relayed. */
export type DevslotLinkDto =
	| { kind: 'devslotUp'; host: string; instance: string }
	| { kind: 'invalid' };

/** A port of the instance that has a tunnel. */
export type DevslotUpPortDto = {
	remotePort: number;
	listenPort: number;
	tunnelId: string;
};

export type DevslotSkipReasonDto =
	| 'localPortBusy'
	| 'needsReview'
	| 'existingTunnel'
	| 'privilegedPort';

export type DevslotSkippedPortDto = {
	remotePort: number;
	reason: DevslotSkipReasonDto;
};

export type DevslotFailedPortDto = { remotePort: number; message: string };

/** A stale route that `devslot.up` removed first. */
export type DevslotRemovedRouteDto = {
	listenPort: number;
	remotePort: number;
	isTemporary: boolean;
};

export type DevslotUpResultDto = {
	host: string;
	instance: string;
	/** Ports whose tunnel was created now. */
	started: DevslotUpPortDto[];
	/** Ports that already had a tunnel, started when it was stopped. */
	existing: DevslotUpPortDto[];
	skipped: DevslotSkippedPortDto[];
	failed: DevslotFailedPortDto[];
	removed: DevslotRemovedRouteDto[];
	/** Stale tunnels kept for the user to decide in the import panel. */
	needsReview: number;
	/** Text the host listed: never a link, never opened from here. */
	appUrl?: string;
	/** Whether the application port is listed by several instances: the browser may reach another one. */
	appPortShared: boolean;
	/** The address the daemon built and opened in the browser. */
	opened?: string;
	/** Why nothing was opened, when opening was asked and failed. */
	openError?: string;
};

/** The pages of the documentation site the app can open. */
export type DocumentationPageDto = 'home' | 'applications';

/** What the banner offers for a newer version. */
export type UpdateActionDto = 'installAndRestart' | 'releaseLinkOnly';

export type UpdateOfferDto = {
	version: string;
	notes?: string;
	releaseUrl: string;
	action: UpdateActionDto;
};

export type UpdateProgressDto =
	| { kind: 'downloading'; downloadedBytes: number; totalBytes?: number }
	| { kind: 'installing' };

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
/** The outcome of looking for an X server on the local display of a host. */
export type X11ProbeDto = {
	display: string;
	isReachable: boolean;
	/** An installed X server that could be started: only set when the display is unreachable and local. */
	server: string | null;
};

export type X11AnchorDto = { host: string; state: X11StateDto };

export type X11LauncherDto = { host: string; name: string; command: string };

export type SnapshotDto = {
	tunnels: TunnelDto[];
	externals: ExternalProcessDto[];
	x11Anchors: X11AnchorDto[];
	x11Launchers: X11LauncherDto[];
	/** Absent when no host overrides its local X display. */
	x11HostDisplays?: Record<string, string>;
	openers: AvailableOpenerDto[];
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
			data: {
				id: string;
				state: TunnelStateDto;
				health: TargetHealthDto;
				isAwake: boolean;
			};
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
	| { event: 'standby.requested'; data: Record<string, never> }
	| { event: 'tray.changed'; data: { status: TrayStatusDto } }
	| { event: 'show.requested'; data: { link?: DevslotLinkDto } }
	| { event: 'externals.changed'; data: { processes: ExternalProcessDto[] } }
	| { event: 'prompt.resolved'; data: { promptId: string } }
	| { event: 'x11.changed'; data: { host: string; state: X11StateDto } };
