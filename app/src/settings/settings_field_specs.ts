import { msg } from '@lingui/core/macro';
import type { MessageDescriptor } from '@lingui/core';

import type { SettingsFormValues } from '#/settings/settings_form_values';

export type SettingsTextField = {
	[TKey in keyof SettingsFormValues]: string extends SettingsFormValues[TKey]
		? TKey
		: never;
}[keyof SettingsFormValues];

export type SettingsToggleField = {
	[TKey in keyof SettingsFormValues]: SettingsFormValues[TKey] extends boolean
		? TKey
		: never;
}[keyof SettingsFormValues];

export type SettingsFieldSpec = {
	field: SettingsTextField;
	label: MessageDescriptor;
	hint?: MessageDescriptor;
	inputMode?: 'decimal' | 'numeric';
	isOptional?: boolean;
};

export type SettingsSectionSpec = {
	name: string;
	legend: MessageDescriptor;
	fields: readonly SettingsFieldSpec[];
};

/** The standby switch is rendered apart, above the idle delay. */
export const STANDBY_SECTION: SettingsSectionSpec = {
	name: 'standby',
	legend: msg`Standby`,
	fields: [
		{
			field: 'standbyIdleSeconds',
			label: msg`Return to standby after (seconds)`,
			hint: msg`Time without connections before the SSH session is released`,
			inputMode: 'numeric',
		},
	],
};

/** Validated text fields in display order; the switches and the start-up tags are rendered apart. */
export const SETTINGS_SECTIONS: readonly SettingsSectionSpec[] = [
	{
		name: 'addresses',
		legend: msg`Addresses`,
		fields: [
			{
				field: 'defaultBindAddress',
				label: msg`Default listen address (local)`,
			},
			{
				field: 'defaultRemoteBindAddress',
				label: msg`Default listen address (remote)`,
			},
		],
	},
	{
		name: 'connection',
		legend: msg`Connection`,
		fields: [
			{
				field: 'keepaliveIntervalSeconds',
				label: msg`Keepalive interval (seconds)`,
				hint: msg`0 to disable`,
				inputMode: 'decimal',
			},
			{
				field: 'keepaliveMax',
				label: msg`Unanswered keepalives tolerated`,
				hint: msg`Beyond this, the connection is considered lost`,
				inputMode: 'numeric',
			},
			{
				field: 'sessionIdleGraceSeconds',
				label: msg`Time before an unused session is closed (seconds)`,
				inputMode: 'decimal',
			},
		],
	},
	{
		name: 'reconnection',
		legend: msg`Reconnection`,
		fields: [
			{
				field: 'reconnectInitialDelaySeconds',
				label: msg`Initial reconnect delay (seconds)`,
				inputMode: 'decimal',
			},
			{
				field: 'reconnectMaxDelaySeconds',
				label: msg`Maximum reconnect delay (seconds)`,
				inputMode: 'decimal',
			},
		],
	},
	{
		name: 'x11',
		legend: msg`X11`,
		fields: [
			{
				field: 'x11Display',
				label: msg`Local X11 display`,
				hint: msg`Empty: DISPLAY variable (127.0.0.1:0.0 on Windows)`,
				isOptional: true,
			},
		],
	},
];
