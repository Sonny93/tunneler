import { i18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import { describe, expect, it } from 'vitest';

import { toSettingsFormValues } from '#/settings/settings_form_values';
import {
	getFirstInvalidSettingsField,
	validateSettingsForm,
} from '#/settings/validate_settings_form';
import { buildSettings } from '#/testing/factories';

const valid = toSettingsFormValues(buildSettings());

const validate = (values: Parameters<typeof validateSettingsForm>[0]) =>
	Object.fromEntries(
		Object.entries(validateSettingsForm(values)).map(([field, error]) => [
			field,
			i18n._(error),
		])
	);

describe('validateSettingsForm', () => {
	it('should accept valid values', () => {
		expect(validateSettingsForm(valid)).toEqual({});
	});

	it('should require the bind addresses', () => {
		expect(
			validate({ ...valid, defaultBindAddress: ' ' }).defaultBindAddress
		).toBe('The address is required.');
	});

	it('should reject a negative or non numeric duration', () => {
		expect(
			validate({ ...valid, keepaliveIntervalSeconds: '-1' })
				.keepaliveIntervalSeconds
		).toBe('The duration must be zero or a positive number of seconds.');
	});

	it('should accept 0 to disable the keepalive', () => {
		expect(
			validateSettingsForm({ ...valid, keepaliveIntervalSeconds: '0' })
		).toEqual({});
	});

	it('should reject a keepalive max that is not an integer', () => {
		expect(validate({ ...valid, keepaliveMax: '1.5' }).keepaliveMax).toBe(
			'The value must be a whole number, zero or more.'
		);
	});

	it('should reject an initial delay above the max delay', () => {
		const errors = validate({
			...valid,
			reconnectInitialDelaySeconds: '120',
			reconnectMaxDelaySeconds: '60',
		});

		expect(errors.reconnectMaxDelaySeconds).toBe(
			'The maximum delay must be greater than or equal to the initial delay.'
		);
	});
});

describe('validateSettingsForm for the standby delay', () => {
	it.each(['0', '-1', '1.5', '', 'abc'])(
		'should reject "%s" as a standby delay',
		(standbyIdleSeconds) => {
			expect(
				validate({ ...valid, standbyIdleSeconds }).standbyIdleSeconds
			).toBe('The delay must be a whole number of seconds greater than 0.');
		}
	);

	it('should accept a whole number of seconds', () => {
		expect(
			validateSettingsForm({ ...valid, standbyIdleSeconds: '45' })
		).toEqual({});
	});
});

const INVALID = msg`invalid`;

describe('getFirstInvalidSettingsField', () => {
	it('should return the first invalid field in form order', () => {
		expect(
			getFirstInvalidSettingsField({
				reconnectMaxDelaySeconds: INVALID,
				keepaliveMax: INVALID,
			})
		).toBe('keepaliveMax');
	});

	it('should return the standby delay after the other fields', () => {
		expect(
			getFirstInvalidSettingsField({
				standbyIdleSeconds: INVALID,
			})
		).toBe('standbyIdleSeconds');
	});

	it('should return null without errors', () => {
		expect(getFirstInvalidSettingsField({})).toBeNull();
	});
});
