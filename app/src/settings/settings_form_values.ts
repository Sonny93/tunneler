import { parseTags } from '#/tunnel_form/parse_tags';
import type {
	OpenHostDto,
	SettingsDto,
	StandbyNotifyDto,
	StandbySoundDto,
} from '#/daemon/protocol';

const MILLISECONDS_PER_SECOND = 1000;
const TAGS_SEPARATOR = ', ';

export type SettingsFormValues = {
	defaultBindAddress: string;
	defaultRemoteBindAddress: string;
	keepaliveIntervalSeconds: string;
	keepaliveMax: string;
	sessionIdleGraceSeconds: string;
	reconnectInitialDelaySeconds: string;
	reconnectMaxDelaySeconds: string;
	launchAtLogin: boolean;
	restoreOnLaunch: boolean;
	startOnLaunchTags: string;
	useOsKeyring: boolean;
	x11Display: string;
	openHost: OpenHostDto;
	standby: boolean;
	standbyIdleSeconds: string;
	standbyNotify: StandbyNotifyDto;
	standbySound: StandbySoundDto;
	checkForUpdates: boolean;
};

export function millisecondsToSeconds(milliseconds: number): string {
	return String(milliseconds / MILLISECONDS_PER_SECOND);
}

/** Seconds typed by the user (comma or dot decimal separator) as whole milliseconds. */
export function secondsToMilliseconds(seconds: string): number {
	const normalized = seconds.trim().replace(',', '.');
	return Math.round(Number(normalized) * MILLISECONDS_PER_SECOND);
}

export function toSettingsFormValues(
	settings: Readonly<SettingsDto>
): SettingsFormValues {
	return {
		defaultBindAddress: settings.defaultBindAddress,
		defaultRemoteBindAddress: settings.defaultRemoteBindAddress,
		keepaliveIntervalSeconds: millisecondsToSeconds(
			settings.keepaliveIntervalMs
		),
		keepaliveMax: String(settings.keepaliveMax),
		sessionIdleGraceSeconds: millisecondsToSeconds(settings.sessionIdleGraceMs),
		reconnectInitialDelaySeconds: millisecondsToSeconds(
			settings.reconnectInitialDelayMs
		),
		reconnectMaxDelaySeconds: millisecondsToSeconds(
			settings.reconnectMaxDelayMs
		),
		launchAtLogin: settings.launchAtLogin,
		restoreOnLaunch: settings.restoreOnLaunch,
		startOnLaunchTags: settings.startOnLaunchTags.join(TAGS_SEPARATOR),
		useOsKeyring: settings.useOsKeyring,
		x11Display: settings.x11Display ?? '',
		openHost: settings.openHost,
		standby: settings.standby,
		standbyIdleSeconds: String(settings.standbyIdleS),
		standbyNotify: settings.standbyNotify,
		standbySound: settings.standbySound,
		checkForUpdates: settings.checkForUpdates,
	};
}

/** Applies valid form values on top of `base`, which keeps any setting the form does not expose. */
export function toSettings(
	values: Readonly<SettingsFormValues>,
	base: Readonly<SettingsDto>
): SettingsDto {
	const x11Display = values.x11Display.trim();
	return {
		...base,
		defaultBindAddress: values.defaultBindAddress.trim(),
		defaultRemoteBindAddress: values.defaultRemoteBindAddress.trim(),
		keepaliveIntervalMs: secondsToMilliseconds(values.keepaliveIntervalSeconds),
		keepaliveMax: Number(values.keepaliveMax.trim()),
		sessionIdleGraceMs: secondsToMilliseconds(values.sessionIdleGraceSeconds),
		reconnectInitialDelayMs: secondsToMilliseconds(
			values.reconnectInitialDelaySeconds
		),
		reconnectMaxDelayMs: secondsToMilliseconds(values.reconnectMaxDelaySeconds),
		launchAtLogin: values.launchAtLogin,
		restoreOnLaunch: values.restoreOnLaunch,
		startOnLaunchTags: parseTags(values.startOnLaunchTags),
		useOsKeyring: values.useOsKeyring,
		x11Display: x11Display === '' ? undefined : x11Display,
		openHost: values.openHost,
		standby: values.standby,
		standbyIdleS: Number(values.standbyIdleSeconds.trim()),
		standbyNotify: values.standbyNotify,
		standbySound: values.standbySound,
		checkForUpdates: values.checkForUpdates,
	};
}
