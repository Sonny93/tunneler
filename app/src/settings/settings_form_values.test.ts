import { describe, expect, it } from 'vitest';

import { buildSettings } from '#/testing/factories';
import {
	secondsToMilliseconds,
	toSettings,
	toSettingsFormValues,
} from '#/settings/settings_form_values';

describe('secondsToMilliseconds', () => {
	it.each([
		['30', 30_000],
		['1.5', 1500],
		['1,5', 1500],
		['0', 0],
	])('should convert "%s" seconds to %i ms', (seconds, expected) => {
		expect(secondsToMilliseconds(seconds)).toBe(expected);
	});
});

describe('settings round trip', () => {
	it('should give back the original settings when nothing changed', () => {
		const settings = buildSettings({ reconnectInitialDelayMs: 1500 });

		expect(toSettings(toSettingsFormValues(settings), settings)).toEqual(
			settings
		);
	});

	it('should keep the check for updates switch when the form turns it off', () => {
		const settings = buildSettings({ checkForUpdates: true });
		const values = {
			...toSettingsFormValues(settings),
			checkForUpdates: false,
		};

		expect(toSettings(values, settings).checkForUpdates).toBe(false);
	});

	it('should show durations in seconds', () => {
		const values = toSettingsFormValues(
			buildSettings({ keepaliveIntervalMs: 30_000 })
		);

		expect(values.keepaliveIntervalSeconds).toBe('30');
	});

	it('should show the start-up tags as a comma-separated text', () => {
		const values = toSettingsFormValues(
			buildSettings({ startOnLaunchTags: ['dev', 'adonis'] })
		);

		expect(values.startOnLaunchTags).toBe('dev, adonis');
	});

	it('should parse the start-up tags text', () => {
		const settings = buildSettings();
		const values = {
			...toSettingsFormValues(settings),
			startOnLaunchTags: ' dev, ,adonis,dev ',
		};

		expect(toSettings(values, settings).startOnLaunchTags).toEqual([
			'dev',
			'adonis',
		]);
	});

	it('should apply the edited switches', () => {
		const settings = buildSettings();
		const values = {
			...toSettingsFormValues(settings),
			launchAtLogin: true,
			restoreOnLaunch: false,
			useOsKeyring: true,
		};

		expect(toSettings(values, settings)).toMatchObject({
			launchAtLogin: true,
			restoreOnLaunch: false,
			useOsKeyring: true,
		});
	});

	it('should keep the settings it is not given values for', () => {
		const settings = buildSettings({ keepaliveMax: 7 });

		expect(toSettings(toSettingsFormValues(settings), settings)).toEqual(
			settings
		);
	});

	it('should show an unset X11 display as an empty field', () => {
		expect(toSettingsFormValues(buildSettings()).x11Display).toBe('');
	});

	it('should trim the X11 display', () => {
		const settings = buildSettings();
		const values = { ...toSettingsFormValues(settings), x11Display: ' :1 ' };

		expect(toSettings(values, settings).x11Display).toBe(':1');
	});

	it('should leave the X11 display unset when the field is emptied', () => {
		const settings = buildSettings({ x11Display: ':1' });
		const values = { ...toSettingsFormValues(settings), x11Display: '  ' };

		expect(toSettings(values, settings).x11Display).toBeUndefined();
	});

	it('should convert edited seconds back to milliseconds', () => {
		const settings = buildSettings();
		const values = {
			...toSettingsFormValues(settings),
			sessionIdleGraceSeconds: '90',
		};

		expect(toSettings(values, settings).sessionIdleGraceMs).toBe(90_000);
	});

	it('should show the standby delay in seconds', () => {
		const values = toSettingsFormValues(buildSettings({ standbyIdleS: 120 }));

		expect(values.standbyIdleSeconds).toBe('120');
	});

	it('should apply the edited standby switch and delay', () => {
		const settings = buildSettings();
		const values = {
			...toSettingsFormValues(settings),
			standby: true,
			standbyIdleSeconds: ' 60 ',
		};

		expect(toSettings(values, settings)).toMatchObject({
			standby: true,
			standbyIdleS: 60,
		});
	});

	it('should apply the edited standby sound', () => {
		const settings = buildSettings();
		const values = {
			...toSettingsFormValues(settings),
			standbySound: 'marimba' as const,
		};

		expect(toSettings(values, settings).standbySound).toBe('marimba');
	});

	it('should apply the edited standby signal', () => {
		const settings = buildSettings();
		const values = {
			...toSettingsFormValues(settings),
			standbyNotify: 'both' as const,
		};

		expect(toSettings(values, settings).standbyNotify).toBe('both');
	});
});
