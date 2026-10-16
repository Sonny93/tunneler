import { describe, expect, it } from 'vitest';

import { buildSettings } from '#/testing/factories';
import { toSettingsFormValues } from '#/settings/settings_form_values';
import {
	getFirstInvalidSettingsField,
	validateSettingsForm,
} from '#/settings/validate_settings_form';

const valid = toSettingsFormValues(buildSettings());

describe('validateSettingsForm', () => {
	it('should accept valid values', () => {
		expect(validateSettingsForm(valid)).toEqual({});
	});

	it('should require the bind addresses', () => {
		expect(
			validateSettingsForm({ ...valid, defaultBindAddress: ' ' })
				.defaultBindAddress
		).toBe("L'adresse est obligatoire.");
	});

	it('should reject a negative or non numeric duration', () => {
		expect(
			validateSettingsForm({ ...valid, keepaliveIntervalSeconds: '-1' })
				.keepaliveIntervalSeconds
		).toBe('La durée doit être un nombre de secondes positif ou nul.');
	});

	it('should accept 0 to disable the keepalive', () => {
		expect(
			validateSettingsForm({ ...valid, keepaliveIntervalSeconds: '0' })
		).toEqual({});
	});

	it('should reject a keepalive max that is not an integer', () => {
		expect(
			validateSettingsForm({ ...valid, keepaliveMax: '1.5' }).keepaliveMax
		).toBe('La valeur doit être un entier positif ou nul.');
	});

	it('should reject an initial delay above the max delay', () => {
		const errors = validateSettingsForm({
			...valid,
			reconnectInitialDelaySeconds: '120',
			reconnectMaxDelaySeconds: '60',
		});

		expect(errors.reconnectMaxDelaySeconds).toBe(
			'Le délai maximal doit être supérieur ou égal au délai initial.'
		);
	});
});

describe('getFirstInvalidSettingsField', () => {
	it('should return the first invalid field in form order', () => {
		expect(
			getFirstInvalidSettingsField({
				reconnectMaxDelaySeconds: 'x',
				keepaliveMax: 'y',
			})
		).toBe('keepaliveMax');
	});

	it('should return null without errors', () => {
		expect(getFirstInvalidSettingsField({})).toBeNull();
	});
});
