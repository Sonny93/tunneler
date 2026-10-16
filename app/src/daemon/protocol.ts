/**
 * Hand-written mirror of the wire DTOs in `crates/ipc/src/protocol/dto.rs` and
 * `event.rs`. The Rust types are the source of truth: the Rust bridge
 * deserializes every message with serde before forwarding it to the webview,
 * so no runtime schema validation happens here. Timestamps are epoch
 * milliseconds; optional fields are omitted when absent.
 */

export type EpochMillis = number;

export type TunnelKindDto = 'local' | 'remote' | 'dynamic';

export type TunnelSpecDto = {
	name: string;
	host: string;
	kind: TunnelKindDto;
	listenPort: number;
	listenAddress?: string;
	targetHost?: string;
	targetPort?: number;
	tags: string[];
};

export type TunnelDefinitionDto = TunnelSpecDto & { id: string };

export type FailureReasonDto =
	| { kind: 'portInUse'; address: string }
	| { kind: 'listenFailed'; address: string; message: string }
	| { kind: 'connect'; message: string }
	| { kind: 'unsupportedKind'; tunnelKind: TunnelKindDto };

export type TunnelStateDto =
	| { kind: 'stopped' }
	| { kind: 'starting' }
	| { kind: 'active' }
	| { kind: 'reconnecting'; attempt: number; nextRetryAt: EpochMillis }
	| { kind: 'failed'; reason: FailureReasonDto };

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
};

export type LogLevelDto = 'info' | 'warn' | 'error';

export type LogEntryDto = {
	at: EpochMillis;
	level: LogLevelDto;
	message: string;
};

export type SettingsDto = {
	defaultBindAddress: string;
	defaultRemoteBindAddress: string;
	keepaliveIntervalMs: number;
	keepaliveMax: number;
	sessionIdleGraceMs: number;
	reconnectInitialDelayMs: number;
	reconnectMaxDelayMs: number;
	healthProbe: boolean;
	restoreOnLaunch: boolean;
	startOnLaunchTags: string[];
	launchAtLogin: boolean;
	x11Display?: string;
};

export type SnapshotDto = {
	tunnels: TunnelDto[];
	settings: SettingsDto;
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
	| { event: 'prompt.resolved'; data: { promptId: string } };
