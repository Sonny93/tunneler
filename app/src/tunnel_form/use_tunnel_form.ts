import { useReducer, useState, type FormEvent } from 'react';

import { useUiStore } from '#/state/ui_store';
import { toTunnelSpec } from '#/tunnel_form/to_tunnel_spec';
import { useToastStore } from '#/shared/toasts/toast_store';
import type { TunnelDefinitionDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import { toTunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import {
	tunnelFormReducer,
	type TunnelFormAction,
} from '#/tunnel_form/tunnel_form_reducer';
import {
	getFirstInvalidField,
	validateTunnelForm,
	type TunnelFormErrors,
} from '#/tunnel_form/validate_tunnel_form';

const DEFAULT_KIND = 'local';

type UseTunnelFormReturn = {
	values: ReturnType<typeof toTunnelFormValues>;
	errors: TunnelFormErrors;
	isSaving: boolean;
	summary: ReturnType<typeof useFormErrorSummary>;
	dispatch: (action: TunnelFormAction) => void;
	submit: (event: FormEvent<HTMLFormElement>) => void;
};

function focusField(form: HTMLFormElement, fieldName: string): void {
	const field = form.elements.namedItem(fieldName);
	if (field instanceof HTMLElement) field.focus();
}

export function useTunnelForm(
	tunnel: Readonly<TunnelDefinitionDto> | null
): UseTunnelFormReturn {
	const gateway = useDaemonGateway();
	const [values, dispatch] = useReducer(
		tunnelFormReducer,
		tunnel,
		toTunnelFormValues
	);
	const [errors, setErrors] = useState<TunnelFormErrors>({});
	const [isSaving, setIsSaving] = useState(false);
	const summary = useFormErrorSummary();

	const save = async () => {
		const spec = toTunnelSpec(values, tunnel?.kind ?? DEFAULT_KIND);
		setIsSaving(true);
		summary.clearFailure();
		try {
			await (tunnel
				? gateway.updateTunnel({ id: tunnel.id, ...spec })
				: gateway.createTunnel(spec));
		} catch (rejection) {
			setIsSaving(false);
			summary.reportFailure(rejection);
			return;
		}
		const outcome = tunnel ? 'enregistré' : 'créé';
		useToastStore.getState().pushToast({
			tone: 'success',
			message: `Tunnel ${spec.name} ${outcome}`,
		});
		useUiStore.getState().closePanel();
	};

	const submit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isSaving) return;
		const foundErrors = validateTunnelForm(values);
		setErrors(foundErrors);
		const firstInvalidField = getFirstInvalidField(foundErrors);
		if (firstInvalidField !== null) {
			focusField(event.currentTarget, firstInvalidField);
			return;
		}
		void save();
	};

	return { values, errors, isSaving, summary, dispatch, submit };
}
