import type { MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import {
	CONNECTION_SECTIONS,
	STANDBY_SECTION,
	type SettingsTextField,
} from '#/settings/settings_field_specs';
import type { SettingsFormValues } from '#/settings/settings_form_values';

export type SettingsFormErrors = Partial<
	Record<SettingsTextField, MessageDescriptor>
>;

const DURATION_PATTERN = /^\d+([.,]\d+)?$/;
const INTEGER_PATTERN = /^\d+$/;

const DURATION_MESSAGE = msg`The duration must be zero or a positive number of seconds.`;
const INTEGER_MESSAGE = msg`The value must be a whole number, zero or more.`;
const STANDBY_IDLE_MESSAGE = msg`The delay must be a whole number of seconds greater than 0.`;
const ADDRESS_MESSAGE = msg`The address is required.`;
const ORDER_MESSAGE = msg`The maximum delay must be greater than or equal to the initial delay.`;

const ADDRESS_FIELDS: readonly SettingsTextField[] = [
	'defaultBindAddress',
	'defaultRemoteBindAddress',
];
const DURATION_FIELDS: readonly SettingsTextField[] = [
	'keepaliveIntervalSeconds',
	'sessionIdleGraceSeconds',
	'reconnectInitialDelaySeconds',
	'reconnectMaxDelaySeconds',
];

function parseSeconds(text: string): number {
	return Number(text.trim().replace(',', '.'));
}

function findFieldErrors(
	values: Readonly<SettingsFormValues>
): SettingsFormErrors {
	const errors: SettingsFormErrors = {};
	for (const field of ADDRESS_FIELDS) {
		if (values[field].trim() === '') errors[field] = ADDRESS_MESSAGE;
	}
	for (const field of DURATION_FIELDS) {
		if (!DURATION_PATTERN.test(values[field].trim())) {
			errors[field] = DURATION_MESSAGE;
		}
	}
	if (!INTEGER_PATTERN.test(values.keepaliveMax.trim())) {
		errors.keepaliveMax = INTEGER_MESSAGE;
	}
	const standbyIdle = values.standbyIdleSeconds.trim();
	if (!INTEGER_PATTERN.test(standbyIdle) || Number(standbyIdle) === 0) {
		errors.standbyIdleSeconds = STANDBY_IDLE_MESSAGE;
	}
	return errors;
}

export function validateSettingsForm(
	values: Readonly<SettingsFormValues>
): SettingsFormErrors {
	const errors = findFieldErrors(values);
	const isOrderCheckable =
		!errors.reconnectInitialDelaySeconds && !errors.reconnectMaxDelaySeconds;
	const isInverted =
		parseSeconds(values.reconnectInitialDelaySeconds) >
		parseSeconds(values.reconnectMaxDelaySeconds);
	if (isOrderCheckable && isInverted) {
		errors.reconnectMaxDelaySeconds = ORDER_MESSAGE;
	}
	return errors;
}

export function getFirstInvalidSettingsField(
	errors: Readonly<SettingsFormErrors>
): SettingsTextField | null {
	const orderedFields = [...CONNECTION_SECTIONS, STANDBY_SECTION].flatMap(
		(section) => section.fields.map((spec) => spec.field)
	);
	return orderedFields.find((field) => errors[field]) ?? null;
}
