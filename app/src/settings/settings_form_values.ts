import type { SettingsDto } from '#/daemon/protocol';

const MILLISECONDS_PER_SECOND = 1000;

export type SettingsFormValues = {
	defaultBindAddress: string;
	defaultRemoteBindAddress: string;
	keepaliveIntervalSeconds: string;
	keepaliveMax: string;
	sessionIdleGraceSeconds: string;
	reconnectInitialDelaySeconds: string;
	reconnectMaxDelaySeconds: string;
	healthProbe: boolean;
	x11Display: string;
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
		healthProbe: settings.healthProbe,
		x11Display: settings.x11Display ?? '',
	};
}

/** Applies valid form values on top of `base`, which keeps the settings the form does not expose. */
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
		healthProbe: values.healthProbe,
		x11Display: x11Display === '' ? undefined : x11Display,
	};
}
