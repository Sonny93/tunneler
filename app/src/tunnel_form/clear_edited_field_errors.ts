import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import type { TunnelFormAction } from '#/tunnel_form/tunnel_form_reducer';
import type { TunnelFormErrors } from '#/tunnel_form/validate_tunnel_form';

function getEditedErrorFields(
	action: Readonly<TunnelFormAction>,
	values: Readonly<TunnelFormValues>
): readonly string[] {
	switch (action.type) {
		case 'textChanged':
			return action.field === 'listenPort' && values.isTargetPortLinked
				? ['listenPort', 'targetPort']
				: [action.field];
		case 'openSchemeChanged':
		case 'openHostChanged':
		case 'openWithChanged':
		case 'standbyChanged':
			return [];
		case 'kindChanged':
		case 'targetPortChanged':
		case 'targetPortRelinked':
			return ['targetPort'];
	}
}

/** Drops the errors of the fields an action edits; other errors stay until the next submit. */
export function clearEditedFieldErrors(
	errors: Readonly<TunnelFormErrors>,
	action: Readonly<TunnelFormAction>,
	values: Readonly<TunnelFormValues>
): TunnelFormErrors {
	const editedFields = getEditedErrorFields(action, values);
	return Object.fromEntries(
		Object.entries(errors).filter(([field]) => !editedFields.includes(field))
	);
}
