import { BROWSER_OPENER_ID } from '#/openers/open_choices';
import {
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

export type OpenerFormErrors = Partial<Record<OpenerFormField, string>>;

const ID_PATTERN = /^[a-z0-9-]+$/u;
const PLACEHOLDER_PATTERN = /\{[^{}]*\}/gu;
const KNOWN_PLACEHOLDERS: readonly string[] = ['{host}', '{port}', '{url}'];
const MIN_PORT = 1;
const MAX_PORT = 65_535;
const PORT_PATTERN = /^\d+$/u;

const PORTS_MESSAGE =
	'Les ports doivent être des entiers entre 1 et 65535, séparés par des virgules.';

function findUnknownPlaceholder(argument: string): string | null {
	const placeholders = argument.match(PLACEHOLDER_PATTERN) ?? [];
	return (
		placeholders.find((found) => !KNOWN_PLACEHOLDERS.includes(found)) ?? null
	);
}

function validateId(id: string, takenIds: readonly string[]): string | null {
	const trimmed = id.trim();
	if (trimmed === '') return "L'identifiant est obligatoire.";
	if (!ID_PATTERN.test(trimmed)) {
		return "L'identifiant ne peut contenir que des minuscules, des chiffres et des tirets.";
	}
	if (trimmed === BROWSER_OPENER_ID) {
		return '« browser » est réservé au navigateur.';
	}
	if (takenIds.includes(trimmed)) {
		return 'Cet identifiant est déjà utilisé par une de vos applications.';
	}
	return null;
}

function validateArguments(text: string): string | null {
	for (const argument of parseArguments(text)) {
		const unknown = findUnknownPlaceholder(argument);
		if (unknown !== null) {
			return `Paramètre inconnu : ${unknown}. Seuls {host}, {port} et {url} existent.`;
		}
	}
	return null;
}

function validatePorts(text: string): string | null {
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
	if (values.name.trim() === '') errors.name = 'Le nom est obligatoire.';
	const idProblem = validateId(values.id, takenIds);
	if (idProblem !== null) errors.id = idProblem;
	if (values.command.trim() === '')
		errors.command = 'La commande est obligatoire.';
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
