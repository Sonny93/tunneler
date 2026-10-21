import { parsePort } from '#/tunnel_form/parse_port';
import { validateTags } from '#/tunnel_form/validate_tags';
import { validateOpenPath } from '#/tunnel_form/validate_open_path';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';

/** Fields that can be invalid, in the order they appear in the form. */
export const TUNNEL_FORM_FIELD_ORDER = [
	'host',
	'listenPort',
	'targetPort',
	'tags',
	'openPath',
] as const;

export type TunnelFormField = (typeof TUNNEL_FORM_FIELD_ORDER)[number];

export type TunnelFormErrors = Partial<Record<TunnelFormField, string>>;

const PORT_MESSAGE = 'Le port doit être un entier entre 1 et 65535.';

function hasUnlinkedInvalidTargetPort(
	values: Readonly<TunnelFormValues>
): boolean {
	if (values.kind === 'dynamic' || values.isTargetPortLinked) return false;
	return parsePort(values.targetPort) === null;
}

export function validateTunnelForm(
	values: Readonly<TunnelFormValues>
): TunnelFormErrors {
	const errors: TunnelFormErrors = {};
	if (values.host.trim() === '') errors.host = "L'hôte est obligatoire.";
	if (parsePort(values.listenPort) === null) errors.listenPort = PORT_MESSAGE;
	if (hasUnlinkedInvalidTargetPort(values)) errors.targetPort = PORT_MESSAGE;
	const tagsProblem = validateTags(values.tags);
	if (tagsProblem !== null) errors.tags = tagsProblem;
	const openPathProblem =
		values.kind === 'local' ? validateOpenPath(values.openPath) : null;
	if (openPathProblem !== null) errors.openPath = openPathProblem;
	return errors;
}

export function getFirstInvalidField(
	errors: Readonly<TunnelFormErrors>
): TunnelFormField | null {
	return TUNNEL_FORM_FIELD_ORDER.find((field) => errors[field]) ?? null;
}
