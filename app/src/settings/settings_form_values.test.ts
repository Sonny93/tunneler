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

	it('should show durations in seconds', () => {
		const values = toSettingsFormValues(
			buildSettings({ keepaliveIntervalMs: 30_000 })
		);

		expect(values.keepaliveIntervalSeconds).toBe('30');
	});

	it('should preserve the settings the form does not expose', () => {
		const settings = buildSettings({
			restoreOnLaunch: false,
			startOnLaunchTags: ['db'],
			launchAtLogin: true,
		});
		const values = { ...toSettingsFormValues(settings), healthProbe: false };

		expect(toSettings(values, settings)).toEqual({
			...settings,
			healthProbe: false,
		});
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
});
