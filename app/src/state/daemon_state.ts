import type { DaemonCommandError } from '#/daemon/daemon_command_error';
import type {
	AvailableOpenerDto,
	CredentialRequestDto,
	ExternalProcessDto,
	HostKeyAlertReasonDto,
	SettingsDto,
	TrayStatusDto,
	TunnelDto,
	X11AnchorDto,
	X11LauncherDto,
} from '#/daemon/protocol';

export type ConnectionState =
	| { status: 'connecting' }
	| { status: 'connected' }
	| { status: 'disconnected'; error: DaemonCommandError };

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
	x11Anchors: readonly X11AnchorDto[];
	x11Launchers: readonly X11LauncherDto[];
	openers: readonly AvailableOpenerDto[];
	settings: SettingsDto | null;
	hosts: readonly string[];
	/** First-in first-out: the first prompt is the one shown. */
	prompts: readonly PendingPrompt[];
	/** Empty when the configuration file is valid. */
	configErrors: readonly string[];
	isQuitRequested: boolean;
	isStandbyRequested: boolean;
	tray: TrayStatusDto;
};

export const INITIAL_DAEMON_STATE: DaemonState = {
	connection: { status: 'connecting' },
	tunnels: [],
	externals: [],
	x11Anchors: [],
	x11Launchers: [],
	openers: [],
	settings: null,
	hosts: [],
	prompts: [],
	configErrors: [],
	isQuitRequested: false,
	isStandbyRequested: false,
	tray: { kind: 'disabled' },
};
