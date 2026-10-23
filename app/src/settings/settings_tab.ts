export const SETTINGS_TABS = [
	'general',
	'connection',
	'standby',
	'opening',
	'secrets',
] as const;

export type SettingsTab = (typeof SETTINGS_TABS)[number];

export const DEFAULT_SETTINGS_TAB: SettingsTab = 'general';
