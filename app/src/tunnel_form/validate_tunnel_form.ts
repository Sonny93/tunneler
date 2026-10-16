import { parsePort } from '#/tunnel_form/parse_port';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';

/** Fields that can be invalid, in the order they appear in the form. */
export const TUNNEL_FORM_FIELD_ORDER = [
	'name',
	'host',
	'listenPort',
	'targetPort',
] as const;

export type TunnelFormField = (typeof TUNNEL_FORM_FIELD_ORDER)[number];

export type TunnelFormErrors = Partial<Record<TunnelFormField, string>>;

const PORT_MESSAGE = 'Le port doit être un entier entre 1 et 65535.';

export function validateTunnelForm(
	values: Readonly<TunnelFormValues>
): TunnelFormErrors {
	const errors: TunnelFormErrors = {};
	if (values.name.trim() === '') errors.name = 'Le nom est obligatoire.';
	if (values.host.trim() === '') errors.host = "L'hôte est obligatoire.";
	if (parsePort(values.listenPort) === null) errors.listenPort = PORT_MESSAGE;
	if (!values.isTargetPortLinked && parsePort(values.targetPort) === null) {
		errors.targetPort = PORT_MESSAGE;
	}
	return errors;
}

export function getFirstInvalidField(
	errors: Readonly<TunnelFormErrors>
): TunnelFormField | null {
	return TUNNEL_FORM_FIELD_ORDER.find((field) => errors[field]) ?? null;
}
