import type { SettingsFormValues } from '#/settings/settings_form_values';
import {
	SETTINGS_SECTIONS,
	STANDBY_SECTION,
	type SettingsTextField,
} from '#/settings/settings_field_specs';

export type SettingsFormErrors = Partial<Record<SettingsTextField, string>>;

const DURATION_PATTERN = /^\d+([.,]\d+)?$/;
const INTEGER_PATTERN = /^\d+$/;

const DURATION_MESSAGE =
	'La durée doit être un nombre de secondes positif ou nul.';
const INTEGER_MESSAGE = 'La valeur doit être un entier positif ou nul.';
const STANDBY_IDLE_MESSAGE =
	'Le délai doit être un nombre entier de secondes supérieur à 0.';
const ADDRESS_MESSAGE = "L'adresse est obligatoire.";
const ORDER_MESSAGE =
	'Le délai maximal doit être supérieur ou égal au délai initial.';

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
	const orderedFields = [...SETTINGS_SECTIONS, STANDBY_SECTION].flatMap(
		(section) => section.fields.map((spec) => spec.field)
	);
	return orderedFields.find((field) => errors[field]) ?? null;
}
