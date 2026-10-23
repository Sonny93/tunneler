import type { MessageDescriptor } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import { BROWSER_OPENER_ID } from '#/openers/open_choices';
import {
	PLACEHOLDER_TEXTS,
	parseArguments,
	type OpenerFormValues,
} from '#/openers/opener_form_values';

/** Fields that can be invalid, in the order they appear in the form. */
export const OPENER_FORM_FIELD_ORDER = [
	'name',
	'id',
	'command',
	'args',
	'ports',
] as const;

export type OpenerFormField = (typeof OPENER_FORM_FIELD_ORDER)[number];

export type OpenerFormErrors = Partial<
	Record<OpenerFormField, MessageDescriptor>
>;

const ID_PATTERN = /^[a-z0-9-]+$/u;
const PLACEHOLDER_PATTERN = /\{[^{}]*\}/gu;
const { hostPlaceholder, portPlaceholder, urlPlaceholder } = PLACEHOLDER_TEXTS;
const KNOWN_PLACEHOLDERS: readonly string[] = Object.values(PLACEHOLDER_TEXTS);
const MIN_PORT = 1;
const MAX_PORT = 65_535;
const PORT_PATTERN = /^\d+$/u;

const PORTS_MESSAGE = msg`The ports must be whole numbers between 1 and 65535, separated by commas.`;
const NAME_MESSAGE = msg`The name is required.`;
const COMMAND_MESSAGE = msg`The command is required.`;
const ID_REQUIRED_MESSAGE = msg`The identifier is required.`;
const ID_FORMAT_MESSAGE = msg`The identifier can only contain lowercase letters, digits and dashes.`;
const ID_RESERVED_MESSAGE = msg`"browser" is reserved for the browser.`;
const ID_TAKEN_MESSAGE = msg`This identifier is already used by one of your applications.`;

function findUnknownPlaceholder(argument: string): string | null {
	const placeholders = argument.match(PLACEHOLDER_PATTERN) ?? [];
	return (
		placeholders.find((found) => !KNOWN_PLACEHOLDERS.includes(found)) ?? null
	);
}

function validateId(
	id: string,
	takenIds: readonly string[]
): MessageDescriptor | null {
	const trimmed = id.trim();
	if (trimmed === '') return ID_REQUIRED_MESSAGE;
	if (!ID_PATTERN.test(trimmed)) return ID_FORMAT_MESSAGE;
	if (trimmed === BROWSER_OPENER_ID) return ID_RESERVED_MESSAGE;
	if (takenIds.includes(trimmed)) return ID_TAKEN_MESSAGE;
	return null;
}

function validateArguments(text: string): MessageDescriptor | null {
	for (const argument of parseArguments(text)) {
		const unknown = findUnknownPlaceholder(argument);
		if (unknown !== null) {
			return msg`Unknown parameter: ${unknown}. Only ${hostPlaceholder}, ${portPlaceholder} and ${urlPlaceholder} exist.`;
		}
	}
	return null;
}

function validatePorts(text: string): MessageDescriptor | null {
	const parts = text
		.split(',')
		.map((part) => part.trim())
		.filter((part) => part !== '');
	const isValid = parts.every((part) => {
		const port = Number(part);
		return PORT_PATTERN.test(part) && port >= MIN_PORT && port <= MAX_PORT;
	});
	return isValid ? null : PORTS_MESSAGE;
}

/** The same rules as the daemon; `takenIds` are the identifiers of the user's other applications. */
export function validateOpenerForm(
	values: Readonly<OpenerFormValues>,
	takenIds: readonly string[]
): OpenerFormErrors {
	const errors: OpenerFormErrors = {};
	if (values.name.trim() === '') errors.name = NAME_MESSAGE;
	const idProblem = validateId(values.id, takenIds);
	if (idProblem !== null) errors.id = idProblem;
	if (values.command.trim() === '') errors.command = COMMAND_MESSAGE;
	const argumentsProblem = validateArguments(values.args);
	if (argumentsProblem !== null) errors.args = argumentsProblem;
	const portsProblem = validatePorts(values.ports);
	if (portsProblem !== null) errors.ports = portsProblem;
	return errors;
}

export function getFirstInvalidOpenerField(
	errors: Readonly<OpenerFormErrors>
): OpenerFormField | null {
	return OPENER_FORM_FIELD_ORDER.find((field) => errors[field]) ?? null;
}
