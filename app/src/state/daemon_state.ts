import type {
	CredentialRequestDto,
	ExternalProcessDto,
	HostKeyAlertReasonDto,
	SettingsDto,
	TrayStatusDto,
	TunnelDto,
} from '#/daemon/protocol';

export type ConnectionState =
	| { status: 'connecting' }
	| { status: 'connected' }
	| { status: 'disconnected'; message: string };

export type PendingPrompt =
	| {
			kind: 'credential';
			promptId: string;
			request: CredentialRequestDto;
	  }
	| {
			kind: 'hostKey';
			promptId: string;
			alias: string;
			lookupName: string;
			algorithm: string;
			fingerprintSha256: string;
	  }
	| {
			kind: 'hostKeyAlert';
			alertId: string;
			alias: string;
			reason: HostKeyAlertReasonDto;
			presentedFingerprint: string;
	  };

export type DaemonState = {
	connection: ConnectionState;
	tunnels: readonly TunnelDto[];
	externals: readonly ExternalProcessDto[];
	settings: SettingsDto | null;
	hosts: readonly string[];
	/** First-in first-out: the first prompt is the one shown. */
	prompts: readonly PendingPrompt[];
	/** Empty when the configuration file is valid. */
	configErrors: readonly string[];
	isQuitRequested: boolean;
	tray: TrayStatusDto;
};

export const INITIAL_DAEMON_STATE: DaemonState = {
	connection: { status: 'connecting' },
	tunnels: [],
	externals: [],
	settings: null,
	hosts: [],
	prompts: [],
	configErrors: [],
	isQuitRequested: false,
	tray: { kind: 'disabled' },
};
